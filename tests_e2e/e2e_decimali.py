"""Block F1: Disegno, mass labels with 0-5 decimals (half-up); an old notebook with labDec 0-4 still works."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
KQ = "document.getElementById('kframe').contentWindow.ketcher"
r = Run(port=8880, wd="/tmp/wd80")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.click("#nav button[data-v=draw]"); pg.wait_for_function("window.TPDraw && TPDraw.ready()", timeout=90000)
        pg.evaluate(KQ + ".setMolecule('CC(=O)Nc1ccc(O)cc1')"); pg.wait_for_timeout(1800)
        txt = lambda: pg.evaluate("(()=>{const d=document.getElementById('kframe').contentDocument;return [...d.querySelectorAll('#mzlab-labels text')].map(t=>t.textContent).join(' ')})()")
        def five():
            assert pg.evaluate("[...document.querySelectorAll('#lb-dec option')].map(o=>o.value).join()") == "0,1,2,3,4,5"
            pg.select_option("#lb-dec", "5"); pg.wait_for_timeout(900)
            t = txt(); assert "151.06333" in t, t                 # paracetamol, M = 151.0633285 -> half-up at 5 decimals
            pg.select_option("#lb-dec", "0"); pg.wait_for_timeout(600); assert "151" in txt() and "151.0" not in txt()
        step("5 decimals: paracetamol M = 151.06333", five)
        def ion():
            pg.select_option("#lb-dec", "5"); pg.evaluate(KQ + ".setMolecule('CC(=O)[NH2+]c1ccc(O)cc1')"); pg.wait_for_timeout(1500)
            t = txt(); assert "152.07060" in t, t                  # [M+H]+ = 151.06332853 + 1.00727646 = 152.07060499
        step("ion m/z with 5 decimals", ion)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
