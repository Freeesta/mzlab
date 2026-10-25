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
RECORD = """() => { window.__pc = {}; window.__over = false;
  const read = () => { for (const c of document.querySelectorAll('#flist td[data-cvrow]')) { const b = c.querySelector('[role=progressbar]'); if (b) (window.__pc[c.dataset.cvrow] = window.__pc[c.dataset.cvrow] || []).push(Number(b.getAttribute('aria-valuenow'))); } };
  new MutationObserver(read).observe(document.querySelector('#flist'), { subtree: true, attributes: true, attributeFilter: ['aria-valuenow'], childList: true });
  window.__tx = new Set(); window.__head = new Set(); window.__live = 0;
  new MutationObserver(() => { for (const x of document.querySelectorAll('#flist [data-cvtxt]')) window.__tx.add(x.textContent.trim()); for (const h of document.querySelectorAll('#flist [data-cvhead]')) window.__head.add(h.textContent);
    window.__live += document.querySelectorAll('#flist [aria-live]').length; }).observe(document.querySelector('#flist'), { subtree: true, characterData: true, childList: true });
  window.__ph = []; addEventListener('mzlab-index', e => window.__ph.push([e.detail.name, e.detail.phase || '', e.detail.frac, performance.now()])); }"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1400}); c.set_default_timeout(120000)
        pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.evaluate(RECORD)
        pg.set_input_files("#pick", MZ + WIFF)
        attendi(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 4 && !PROGRESSO.busy()", 120000)
        pcs = pg.evaluate("window.__pc")
        print("   bar:", pcs)
        def forward():
            assert pcs, pcs
            for name, v in pcs.items():
                if not name.endswith('.mzML'): continue                                  # the row of a maker's pair is replaced by the rows of its samples
                assert v == sorted(v), (name, v)                                       # forward only
                first = v.index(100) if 100 in v else len(v)
                assert all(x <= 99 for x in v[:first]), (name, v)                         # never 100 before the end of the file (the last row to end is not painted again)
            assert any(len(v) >= 3 for v in pcs.values()), pcs
        step("the bar of each row only goes forward and is not 100 before the end of its file", forward)
        def one_bar():
            assert pg.evaluate("document.querySelectorAll('#up .loadbar').length") == 0, "the card has no bar of its own: the rows carry the progress"
            assert pg.evaluate("document.querySelectorAll('#up .prog').length") == 0, "a second bar is left"
        step("one system: no bar in the card", one_bar)
        def texts():
            tx = pg.evaluate("[...window.__tx]"); head = pg.evaluate("[...window.__head]"); print("   texts:", tx, "heads:", head[:3])
            import re
            assert any(t.startswith(("Loading", "Carico")) for t in tx), tx
            assert any(re.fullmatch(r"(Reading [\d.,\u00a0\u202f ]+ of [\d.,\u00a0\u202f ]+ scans|Leggo [\d.,\u00a0\u202f ]+ / [\d.,\u00a0\u202f ]+ scansioni)", t) for t in tx), tx      # the scans read of the sample, from the engine
            assert any(re.fullmatch(r"(\d+ of \d+ ready|\d+ di \d+ pronti) · \d+:\d\d", h) for h in head), head
        step("each row says what it does (Loading, Reading n of m scans) and the head says «k of n ready · m:ss»", texts)
        def live():
            assert pg.evaluate("window.__live") == 0, "no aria-live inside the list: only the summary region speaks"
            assert pg.evaluate("document.querySelectorAll('#cvlive[role=status][aria-live=polite]').length") == 1
            assert pg.evaluate("document.getElementById('cvlive').textContent") != ""
        step("one polite region for the summary, none in the rows", live)
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
