"""A SIM (Q1MI) .wiff of a triple quadrupole on the built site with the Rust engine (synthetic pair of tools/wiff_sim.py, no laboratory data): the two samples
open as two transition files (kind «mrm»: the ions are the transitions, Q1 only and no Q3), every monitored ion is its own SIM channel labelled by m/z, polarity, dwell and name (none dropped, none read as an MRM transition) and
the engine answers every request. Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_wiff_sim.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="wiff_sim_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_sim.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "sim.json").read_text())
WANT = sorted(f"SIM {i['q1']} ({'+' if i['polarita'] == 1 else '−'}) {i['dwell']:g} ms «{i['name']}»" for i in TRUE["ioni"])
r = Run(port=8874, wd="/tmp/wd_wiff_sim")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [D / "sim.wiff", D / "sim.wiff.scan"]); pg.click("text=Carica dati"); ready(pg); attendi(pg, "E.files.length >= 2", timeout=120000)
        def channels():
            for k in pg.evaluate("E.files.map(f => f.k)"):
                got = pg.evaluate(f"CANALI.canali({k}).then(j => j.canali.filter(c => c.fam === 'sim').map(c => c.et))")
                assert sorted(got) == WANT, (k, got, WANT)
                srm = pg.evaluate(f"CANALI.canali({k}).then(j => j.canali.filter(c => c.fam === 'srm').length)")
                assert srm == 0, (k, srm)
        step("every SIM ion of both samples is one labelled channel, none an MRM transition", channels)
        def transitions():
            want = sorted(i["q1"] for i in TRUE["ioni"])
            assert pg.evaluate("E.files.map(f => f.kind)") == ["mrm", "mrm"], pg.evaluate("E.files.map(f => f.kind)")
            for k in pg.evaluate("E.files.map(f => f.k)"):
                tr = pg.evaluate(f"fetch('api/mrm?k={k}').then(r => r.json()).then(j => j.files[0].transitions.map(t => [t.q1, t.q3, t.name, t.y.length]))")
                assert sorted(t[0] for t in tr) == want and all(t[1] is None and t[3] == TRUE["cicli"] for t in tr), (k, tr)
        step("the ions are the transitions of the file, Q1 only", transitions)
        step("the engine answered every request", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
