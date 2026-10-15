"""Loading errors: a short message with the cause, a closed «Details» button (aria-expanded) with the phase, the format, the original error and the header (never the data), and «Copy diagnosis».
Real failures of the engine: a truncated mzML and a file that is not XML (synthetic, tools/mzml_msconvert_sintetico.py). Needs the site and the .wasm. Run from the repo root: python3 tests_e2e/e2e_diagnosi.py"""
import os, shutil, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

PORT = 8851
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py + wasm")); sys.exit(0)
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_sintetico as gen
D = Path(tempfile.mkdtemp(prefix="diag_e2e_"))
gen.scrivi(D / "ok.mzML", scans=30, peaks1=40, peaks2=10)
whole = (D / "ok.mzML").read_bytes()
(D / "troncato.mzML").write_bytes(whole[: len(whole) // 2])
(D / "nonxml.mzML").write_bytes(b"\x00\x01\x02 this is not xml " * 400)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        for lang, det, copy in (("it-IT", "Dettagli", "Copia diagnosi"), ("en-US", "Details", "Copy diagnosis")):
            c = b.new_context(locale=lang, viewport={"width": 1400, "height": 1200}, permissions=["clipboard-read", "clipboard-write"]); c.set_default_timeout(120000)
            pg = c.new_page(); pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
            pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop"); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            pg.set_input_files("#pick", [str(D / "troncato.mzML"), str(D / "nonxml.mzML")])
            pg.wait_for_selector("#unopen:not([hidden]) .diag", timeout=120000)
            def shown():
                t = pg.inner_text("#unopen"); btns = pg.locator("#unopen [data-a=diagdet]")
                assert btns.count() >= 1 and det in btns.first.inner_text(), t
                assert btns.first.get_attribute("aria-expanded") == "false" and not pg.locator("#unopen .diagbox").first.is_visible()
                assert "troncato" in t or "nonxml" in t
            step(f"{lang}: the notice has a closed «{det}» button", shown)
            def open_it():
                pg.locator("#unopen [data-a=diagdet]").first.click()
                assert pg.locator("#unopen [data-a=diagdet]").first.get_attribute("aria-expanded") == "true" and pg.locator("#unopen .diagbox").first.is_visible()
                box = pg.locator("#unopen .diagbox").first.inner_text()
                assert copy in box and ("•" not in box)
                assert "/tmp" not in box and "diag_e2e" not in box, box
            step(f"{lang}: the details open (phase, format, error)", open_it)
            def copied():
                pg.locator("#unopen [data-a=diagcopy]").first.click()
                pg.wait_for_function("document.querySelector('#unopen [data-a=diagcopy]').textContent !== %r" % copy)
                txt = pg.evaluate("navigator.clipboard.readText()")
                assert "phase:" in txt and "error:" in txt and "troncato" not in txt and "nonxml" not in txt, txt
            step(f"{lang}: «{copy}» puts the diagnosis on the clipboard, without file names", copied)
            c.close()
        assert not errs, errs
        b.close()
finally:
    srv.terminate(); shutil.rmtree(D, ignore_errors=True)
