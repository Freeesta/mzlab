"""A scan outside the file (synthetic Full Scan mzML of synth.py): the engine refuses it with its reason (err.scan.outsideN, err.scan.outside, ...),
the bridge answers 400 with that key, as the desktop does, never 501 "operation not available" (hrscans.rs, rust-scans.js). Needs the site and the
.wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_scansione_fuori.py"""
import json, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="fuori_"))
synth.write_full(D / "P.mzML", 0, seed=1)
INIT = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
ASK = "q => fetch(q).then(async x => [x.status, await x.text()])"

r = Run(port=8973, wd="/tmp/wd_scansione_fuori")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); c = b.new_context(viewport={"width": 1400, "height": 900}); c.set_default_timeout(120000); c.add_init_script(INIT)
        A = c.new_page(); A.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        A.goto(f"http://127.0.0.1:{r.port}/"); A.wait_for_selector("#drop", state="attached")
        attendi(A, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=120000)
        A.set_input_files("#pick", [str(D / "P.mzML")])
        attendi(A, "ST.files.length === 1 && ST.files.every(f => f.kind) && UPBUSY === 0 && OPENNOW.busy() === 0", timeout=120000)
        n = json.loads(A.evaluate(ASK, "api/spectra?k=0&level=1&i0=0&i1=0")[1])["n"]
        def refused():
            for q, key, params in ((f"api/spectra?k=0&level=1&i0={n}&i1={n}", "err.scan.outsideN", {"n": str(n)}), ("api/spectra?k=0&level=1&i0=3&i1=2", "err.scan.range", {}),
                                   ("api/scan?k=0&sid=99999", "err.scan.outside", {}), ("api/scanavg?k=0&sids=99999", "err.scan.noneValid", {}),
                                   ("api/scaninfo?k=0&sid=99999", "err.scan.outside", {}), (f"api/scanbin?k=0&level=1&i0={n}&i1={n}", "err.scan.outsideN", {"n": str(n)})):
                st, text = A.evaluate(ASK, q)
                assert st == 400 and json.loads(text) == {"error_key": key, "params": params}, (q, st, text)
        step("a scan outside the file is refused with its reason (400, its key)", refused)
        step("no request went unanswered", lambda: (lambda m: m and (_ for _ in ()).throw(AssertionError(m)))(A.evaluate("window.MZLAB_LR_MISS || []")))
        def inside():
            for q in ("api/spectra?k=0&level=1&i0=0&i1=1", "api/scan?k=0&sid=0", "api/scanavg?k=0&sids=0,1", "api/scaninfo?k=0&sid=0"):
                st, _ = A.evaluate(ASK, q); assert st == 200, (q, st)
        step("a scan inside the file is answered as before", inside)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
bad = [n for n, s in steps if s != "ok"]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report() if bad else None
sys.exit(1 if bad else 0)
