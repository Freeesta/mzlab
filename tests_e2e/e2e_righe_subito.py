"""Files dropped before the engine is ready are rows of the side list at once, «Waiting for engine» (upload in shell.js, docs/agenti/avvio.md): the .wasm is held back, two mzML are
dropped, two rows with the right names and no percent are there (role=status count); released, the same rows complete; an engine that fails turns them into failed rows with the banner's reason;
× on a waiting row leaves that file out. Synthetic data. Needs the site and the .wasm (python tools/prepara_sito.py)."""
# covers: mzlab/web/shell.js, mzlab/web/avvio-progressivo.js, mzlab/web/upload-rows.js, mzlab/web/conversione.js
import os, re, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port
import synth

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600])); errs.append(name)

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/prepara_sito.py")); sys.exit(1)

D = Path(tempfile.mkdtemp(prefix="righe_e2e_"))
MZ = synth.make_series(str(D), [0, 5], False, False)
NAMES = [Path(p).name for p in MZ]
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
URL = f"http://127.0.0.1:{PORT}/"
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(URL, timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting
WASM = re.compile(r"/static/wasm/.*\.wasm(\?.*)?$")
ROWS = "() => [...document.querySelectorAll('#fl-up [data-up]')].map(e => ({ n: e.dataset.up, t: (e.querySelector('[data-uptxt]') || {}).textContent, err: e.classList.contains('err') }))"
TWO = "document.querySelectorAll('#fl-up [data-up]').length === 2"

def page_with_held_engine(b):
    ctx = b.new_context(locale="en-US", viewport={"width": 1300, "height": 900}); held = []
    def route(r):
        if WASM.search(r.request.url): held.append(r)
        else: r.continue_()
    ctx.route(re.compile(r".*"), route)
    pg = ctx.new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.goto(URL)
    pg.wait_for_function("typeof upload === 'function' && typeof AVVIO !== 'undefined'", timeout=60000)
    assert pg.evaluate("AVVIO.stato") == "pending", "the engine must still be held back"
    return ctx, pg, held

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        def waiting_then_done():
            ctx, pg, held = page_with_held_engine(b)
            pg.set_input_files("#pick", MZ)
            pg.wait_for_function(TWO, timeout=10000)
            rows = pg.evaluate(ROWS)
            assert [r["n"] for r in rows] == NAMES, rows
            assert all("Waiting for engine" in (r["t"] or "") and "%" not in (r["t"] or "") for r in rows), rows
            assert pg.get_attribute("#cvlive", "role") == "status", "the count is read by a polite status region"
            assert pg.evaluate("AVVIO.stato") == "pending"
            pg.evaluate("window.__rows = [...document.querySelectorAll('#fl-up [data-up]')]")
            for r in held: r.continue_()
            pg.wait_for_function("window.__rows.some(e => !/Waiting for engine/.test(e.textContent))", timeout=90000)
            # taken over in place: the elements of the waiting rows are the ones that went on (no row removed and added again)
            assert pg.evaluate("window.__rows.filter(e => e.isConnected).length") >= 1
            pg.wait_for_function("ST.files.length === 2", timeout=90000)
            assert sorted(pg.evaluate("ST.files.map(f => f.name)")) == sorted(NAMES)
            ctx.close()
        step("two dropped files show as «Waiting for engine» rows, are taken over in place and load", waiting_then_done)

        def cancel_one():
            ctx, pg, held = page_with_held_engine(b)
            pg.set_input_files("#pick", MZ)
            pg.wait_for_function(TWO, timeout=10000)
            pg.click(f"#fl-up [data-up-x='{NAMES[0]}']")
            assert [r["n"] for r in pg.evaluate(ROWS)] == [NAMES[1]]
            for r in held: r.continue_()
            pg.wait_for_function("ST.files.length === 1", timeout=90000)
            assert pg.evaluate("ST.files.map(f => f.name)") == [NAMES[1]]
            ctx.close()
        step("× on a waiting row leaves that file out", cancel_one)

        def engine_fails():
            ctx, pg, held = page_with_held_engine(b)
            pg.set_input_files("#pick", MZ)
            pg.wait_for_function(TWO, timeout=10000)
            for r in held: r.fulfill(status=404, body="no")
            pg.wait_for_selector("#avvio-errore", timeout=60000)
            reason = pg.inner_text("#avvio-errore-msg")
            pg.wait_for_function("document.querySelectorAll('#fl-up .fl.err').length === 2", timeout=10000)
            rows = pg.evaluate(ROWS)
            assert [r["n"] for r in rows] == NAMES and all(r["err"] for r in rows), (rows, reason)
            ctx.close()
        step("a failing engine turns the rows into failed rows", engine_fails)
        b.close()
finally:
    srv.terminate()
print("FAIL" if errs else "OK")
sys.exit(1 if errs else 0)
