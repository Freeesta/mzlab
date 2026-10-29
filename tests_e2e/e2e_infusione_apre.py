"""Loading an MSn infusion file opens the precursor tree in the side bar (not the file list); loading an LC file keeps the file list. Synthetic files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_infusione_apre.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("infapre")
F = write_msn(D / "MSn_synth_t03.mzML", caffeine_nodes(), n_scans=12)
r = Run(port=8980, wd="/tmp/wd_infapre")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1200})
        load(pg, [F], 6000, 1)
        attendi(pg, "BARRA.curTab === 'alb'", 30000)
        def inf():
            assert pg.evaluate("BARRA.isOpen('alb')"), pg.evaluate("BARRA.curTab")
            pg.wait_for_selector("#inf-tree tbody tr", timeout=30000)
            assert pg.evaluate("E.panels.length") > 0, "the cells stay"
        step("an infusion file opens the precursor tree, not the file list", inf)
        pg2 = r.b.new_page(viewport={"width": 1500, "height": 1200}); pg2.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg2, "window.qqReady === true")
        load(pg2, [D / "B_FullMass-t0.mzML"], 4000, 1)
        def lc():
            assert pg2.evaluate("BARRA.curTab") == "file", pg2.evaluate("BARRA.curTab")
        step("an LC file keeps the file list", lc)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
