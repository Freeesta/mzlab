"""A file with "#" in its name (synthetic: a Full Scan mzML and a .wiff pair of tools/wiff_cromatogrammi.py, loaded with two files without one): "#" separates a
file from its items in the list, so it enters with "_" (hashFree, conversione.js), the notice names the old and the new name once per file, and every file
opens, also after a reload (it used to stay listed but never open once another file was there). Needs the site and the .wasm (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_nome_cancelletto.py"""
import json, shutil, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="cancelletto_"))
W = D / "w"
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_cromatogrammi.py"), str(W)], check=True, capture_output=True)
synth.write_full(D / "P#1.mzML", 0, seed=1); synth.write_full(D / "Q.mzML", 10, seed=2)
for x in ("", ".scan"):
    shutil.copy(W / f"fsmrm.wiff{x}", D / f"A#1.wiff{x}"); shutil.copy(W / f"duemrm.wiff{x}", D / f"B.wiff{x}")
FILES = [str(D / n) for n in ("P#1.mzML", "Q.mzML", "A#1.wiff", "A#1.wiff.scan", "B.wiff", "B.wiff.scan")]
WANT = ["A_1.mzML", "B.mzML", "P_1.mzML", "Q.mzML"]
INIT = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
DONE = f"ST.files.length === {len(WANT)} && ST.files.every(f => f.kind) && UPBUSY === 0 && OPENNOW.busy() === 0"
LISTED = "ST.files.map(f => f.name).sort()"

r = Run(port=8989, wd="/tmp/wd_cancelletto")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); c = b.new_context(viewport={"width": 1400, "height": 900}); c.set_default_timeout(120000); c.add_init_script(INIT)
        def opened(pg):
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop", state="attached"); wait_for_js(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=120000)
            return pg
        A = opened(c.new_page())
        A.set_input_files("#pick", FILES); wait_for_js(A, DONE, timeout=120000)
        def listed():
            assert A.evaluate(LISTED) == WANT, A.evaluate(LISTED)
        step("the files with # enter with _", listed)
        def notice():
            t = A.evaluate("document.getElementById('unopen').textContent")
            for old, new in (("P#1.mzML", "P_1.mzML"), ("A#1.wiff", "A_1.wiff")):
                assert t.count(old) == 1 and new in t, t
            assert "A#1.wiff.scan" not in t, t
        step("the notice names the old and the new name, once per .wiff pair", notice)
        def every_open():
            wait_for_js(A, f"{json.dumps(WANT)}.every(n => E.files.some(f => !f.gone && String(f.file).split('#')[0] === n))", timeout=60000)
        step("every file opens", every_open)
        A.close(); B = opened(c.new_page())
        def after_reload():
            wait_for_js(B, DONE, timeout=120000)
            assert B.evaluate(LISTED) == WANT, B.evaluate(LISTED)
            wait_for_js(B, f"{json.dumps(WANT)}.every(n => E.files.some(f => !f.gone && String(f.file).split('#')[0] === n))", timeout=60000)
        step("after a reload the same files are listed and open", after_reload)
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
