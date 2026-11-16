"""The choice of the samples of a big SCIEX .wiff + .wiff.scan in the browser (synthetic pair of tools/wiff_grande.py, no laboratory data): over the threshold (lowered here with
window.MZLAB_WIFF_PICK_BYTES) a dialog lists the samples with none ticked; «Open selected» is disabled until one is ticked and only the ticked samples enter the list; «Open all» loads all;
«Cancel» loads none; a pair under the threshold opens without the dialog. Needs the built site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_wiff_scelta.py"""
# covers: mzlab/web/wiff-scelta.js
import os, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python3 tools/prepara_sito.py")); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="wiff_scelta_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(D), "--name", "lotto", "--samples", "4", "--scans", "60", "--points", "300"], check=True, capture_output=True)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting
PAIR = [str(D / "lotto.wiff"), str(D / "lotto.wiff.scan")]
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        def page(threshold):
            c = b.new_context(locale="en-US", viewport={"width": 1500, "height": 1200}); c.set_default_timeout(120000)      # a context of its own: nothing of the pair is cached from the scenario before
            pg = c.new_page(); pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
                               + (f"window.MZLAB_WIFF_PICK_BYTES={threshold};" if threshold is not None else ""))
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop"); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            return pg
        files = "ST.files.map(f => f.name).sort().join('|')"
        pg = page(1000)
        pg.set_input_files("#pick", PAIR); pg.wait_for_selector("dialog.wpick-dlg[open]")
        def dialog():
            assert pg.locator(".wpick-list input").count() == 4
            assert pg.locator(".wpick-list input:checked").count() == 0, "nothing is ticked by default"
            assert pg.locator(".wpick-dlg [role=status]").inner_text() == "0 of 4 chosen"
            assert pg.locator(".wpick-dlg button", has_text="Open selected").is_disabled()
        step("the dialog lists the 4 samples, none ticked, «Open selected» disabled", dialog)
        pg.locator(".wpick-list input").nth(1).check(); pg.locator(".wpick-list input").nth(3).check()
        step("the status counts the ticked samples", lambda: (pg.locator(".wpick-dlg [role=status]").inner_text() == "2 of 4 chosen") or (_ for _ in ()).throw(AssertionError("count")))
        pg.locator(".wpick-dlg button", has_text="Open selected").click()
        pg.wait_for_function("UPBUSY === 0 && ST.files.length === 2", timeout=120000)
        step("only the two ticked samples enter the list", lambda: (pg.evaluate(files) == "lotto_s2.mzML|lotto_s4.mzML") or (_ for _ in ()).throw(AssertionError(pg.evaluate(files))))
        pg.close()

        pg = page(1000); pg.set_input_files("#pick", PAIR); pg.wait_for_selector("dialog.wpick-dlg[open]")
        pg.locator(".wpick-dlg button", has_text="Open all").click(); pg.wait_for_function("UPBUSY === 0 && ST.files.length === 4", timeout=120000)
        step("«Open all» loads the 4 samples", lambda: None); pg.close()

        pg = page(1000); pg.set_input_files("#pick", PAIR); pg.wait_for_selector("dialog.wpick-dlg[open]")
        pg.keyboard.press("Escape"); pg.wait_for_selector("dialog.wpick-dlg", state="detached")
        pg.wait_for_function("UPBUSY === 0", timeout=60000)
        step("Escape cancels: nothing is loaded", lambda: (pg.evaluate("ST.files.length") == 0) or (_ for _ in ()).throw(AssertionError("loaded"))); pg.close()

        pg = page(None); pg.set_input_files("#pick", PAIR); pg.wait_for_function("UPBUSY === 0 && ST.files.length === 4", timeout=120000)
        step("a pair under the threshold opens with no dialog", lambda: (pg.locator("dialog.wpick-dlg").count() == 0) or (_ for _ in ()).throw(AssertionError("dialog"))); pg.close()
        b.close()
finally:
    srv.terminate()
for e in errs: print("  ", ("page error", e))
