"""Automatic background of the spectrum: «background of a stretch» with nothing typed picks a stretch from the TIC (after waste and front), shows it, lets the student choose another or type one.
Synthetic series SUB_t0/t10/t20 (see e2e_sottrazione)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from synth import make_subtraction
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_fondo_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
files = make_subtraction("/tmp/wd_fondosp_files", blank=False)
r = Run(port=8968, wd="/tmp/wd_fondosp")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, files); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1,r0:5.0,r1:5.4}); "); ready(pg)
        P = "E.panels.find(q=>q.type==='spec')"
        def auto():
            pg.evaluate(f"{P}.bg='w'; ctl({P}); draw({P})"); ready(pg); pg.wait_for_timeout(800)
            a = pg.evaluate(f"{P}._bgauto"); assert a and a["cands"] and a["lead"] >= 0.5, a
            assert all(c[0] >= a["lead"] and c[1] > c[0] for c in a["cands"]), a
            assert "automatic" in pg.evaluate(f"{P}.leg.textContent"), pg.evaluate(f"{P}.leg.textContent")
            assert pg.locator("[data-o=bwi]").count() == 1
        step("no stretch typed: the program picks one after the front, writes it in the legend and offers the candidates", auto)
        def other():
            n = pg.locator("[data-o=bwi] option").count()
            if n > 1:
                pg.select_option("[data-o=bwi]", "1"); ready(pg); pg.wait_for_timeout(500)
                a = pg.evaluate(f"{P}._bgauto"); t = pg.evaluate(f"{P}.leg.textContent")
                assert f"{a['cands'][1][0]}" in t, (a, t)
        step("a click on another candidate changes the stretch", other)
        def typed():
            pg.evaluate(f"{P}.bw0=9; {P}.bw1=9.5; ctl({P}); draw({P})"); ready(pg); pg.wait_for_timeout(500)
            assert pg.locator("[data-o=bwauto]").count() == 1 and "automatic" not in pg.evaluate(f"{P}.leg.textContent")
            pg.locator("[data-o=bwauto]").click(); ready(pg); pg.wait_for_timeout(600)
            assert pg.evaluate(f"{P}.bw0") is None and pg.locator("[data-o=bwi]").count() == 1
        step("a typed stretch wins; the ↺ button goes back to automatic", typed)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
