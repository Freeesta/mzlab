"""One bar for every loading: loading a .mzML series and a SCIEX .wiff pair together, the bar of the loading card (role=progressbar in #up) only goes forward, is not 100 before the end, and the
shares of the phases are printed (the weights of progresso.js). Synthetic data. Needs the site and the .wasm (python tools/build_site.py + the CI build). Run: python3 tests_e2e/e2e_progresso.py"""
import os, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, attendi
import synth

from lib import free_port; PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py + the .wasm")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="prog_e2e_"))
synth.make_series(str(D), [0, 5, 10], False, False)
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting
MZ = sorted(str(p) for p in D.glob("*.mzML"))[:3]
WIFF = [str(D / n) for n in ["serie.wiff", "serie.wiff.scan"]]
RECORD = """() => { window.__pc = []; window.__over = false;
  const read = () => { const b = document.querySelector('#up .loadbar [role=progressbar]'); if (b) window.__pc.push(Number(b.getAttribute('aria-valuenow'))); };
  new MutationObserver(read).observe(document.querySelector('#up'), { subtree: true, attributes: true, attributeFilter: ['aria-valuenow'], childList: true });
  window.__ph = []; addEventListener('mzlab-index', e => window.__ph.push([e.detail.name, e.detail.phase || '', e.detail.frac, performance.now()])); }"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1400}); c.set_default_timeout(120000)
        pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.evaluate(RECORD)
        pg.set_input_files("#pick", MZ + WIFF)
        attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 4 && !PROGRESSO.busy()", 120000)
        pcs = pg.evaluate("window.__pc")
        print("   bar:", pcs)
        def forward():
            assert len(pcs) >= 3, pcs
            assert pcs == sorted(pcs), pcs                                          # forward only
            assert pcs[-1] == 100 and all(x <= 99 for x in pcs[:-1]), pcs
        step("the bar of the loading card only goes forward and is not 100 before the end", forward)
        def one_bar():
            assert pg.evaluate("document.querySelectorAll('#up .loadbar').length") == 0      # removed when the last loading is over
            assert pg.evaluate("document.querySelectorAll('#up .prog').length") == 0, "a second bar is left"
        step("one bar, removed at the end", one_bar)
        ph = pg.evaluate("window.__ph")
        def weights():
            by = {}
            for name, phase, frac, t in ph: by.setdefault(name, []).append((phase, frac, t))
            for name, ev in by.items():
                fr = [e[1] for e in ev]
                print("   ", name, "events", len(ev), "phases", sorted({e[0] for e in ev}), "last frac", round(fr[-1], 3))
            assert by, "no mzlab-index event"
        step("the readers tell their phases (printed)", weights)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
