"""RT x m/z map: the MS / PDA selector appears only for a file that has a PDA matrix. A file with just a PDA trace (the synthetic ones) and the browser engine
have none, so the map shows the plain MS level and the PDA trace stays in the chromatogram."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_pda.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "mappa_pda_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "MAPPDA_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(5))
r = Run(port=8866, wd="/tmp/wd_mappa_pda")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)])
        pg.click("text=Carica dati"); ready(pg)
        def trace_only():
            assert pg.evaluate("E.files[0].pda") is True
            pg.click("#np-map"); ready(pg); attendi(pg, "E.files[0]._pdaMap === false")
            assert pg.evaluate(f"!{M}.el.querySelector('select[data-o=mapKind]')"), "PDA offered without a matrix"
            assert pg.evaluate(f"{M}.el.querySelector('.mzlv').textContent.startsWith('MS')")
        step("a PDA trace without matrix: no MS / PDA selector on the map", trace_only)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
