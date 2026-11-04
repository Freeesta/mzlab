"""Two tabs open the same .wiff (synthetic, tools/wiff_misto.py; no laboratory data): the second tab waits for the index the first one is building (Web Lock
"mzlab-job:idx/<name>") instead of opening the same OPFS handle, which used to fail with NoModificationAllowedError, restart the engine and leave samples
unread (LR_MISS, 501). The test stands in for the first tab by holding the locks itself, so it does not depend on timing.
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_due_tab_indice.py"""
import json, subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

D = Path(tempfile.mkdtemp(prefix="due_tab_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_misto.py"), str(D)], check=True, capture_output=True)
PAIR = [str(D / "misto.wiff"), str(D / "misto.wiff.scan")]
N = len(json.loads((D / "tipi.json").read_text()))
INIT = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
DONE = f"ST.files.length === {N} && ST.files.every(f => f.kind) && UPBUSY === 0 && OPENNOW.busy() === 0"
HOLD = """names => { window.__held = 0; window.__free = null; const gate = new Promise(r => { window.__free = r; });
  for (const n of names) navigator.locks.request('mzlab-job:idx/' + n, () => { window.__held++; return gate; }); }"""

r = Run(port=8830 + 140, wd="/tmp/wd_due_tab")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); c = b.new_context(viewport={"width": 1400, "height": 900}); c.set_default_timeout(120000); c.add_init_script(INIT)
        pages = []
        for i in range(2):
            pg = c.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#drop"); attendi(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange")
            pages.append(pg)
        A, B = pages
        A.set_input_files("#pick", PAIR); attendi(A, DONE, timeout=120000)
        names = A.evaluate("ST.files.map(f => f.name)")
        step("tab A reads every sample", lambda: (len(names) == N) or (_ for _ in ()).throw(AssertionError(names)))
        A.evaluate(HOLD, names); attendi(A, f"window.__held === {N}")
        B.set_input_files("#pick", PAIR)
        def waits():
            B.wait_for_function("false", timeout=3000) if False else None
            for _ in range(10):
                try: B.wait_for_function("false", timeout=300)
                except Exception: pass
            assert B.evaluate("typeof UPBUSY !== 'undefined' && (UPBUSY > 0 || OPENNOW.busy() > 0 || !ST.files.every(f => f.kind))"), "tab B finished while tab A held the index locks"
            assert not B.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), B.evaluate("window.MZLAB_LR_MISS")
        step("tab B waits while tab A holds the index (no error)", waits)
        A.evaluate("window.__free()")
        def finishes():
            attendi(B, DONE, timeout=120000)
            assert not B.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), B.evaluate("window.MZLAB_LR_MISS")
            assert B.evaluate("document.querySelectorAll('#fl-up .err, #fl-up [data-err]').length") == 0
        step("tab B reads every sample once tab A is done", finishes)
        b.close()
finally:
    r.close()
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"] + [("page error", str(e)[:300]) for e in r.errs if e[0] == "pageerror"]
sys.exit(1 if bad else 0)
