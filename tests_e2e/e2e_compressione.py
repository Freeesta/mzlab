"""The three compressions of «Download mzML» on the built site (Rust engine, no Pyodide): a SCIEX .wiff pair of tools/wiff_riferimento.py is loaded, its mzML is written on request
with Standard, Smallest and Uncompressed (the select of the Advanced block), the files saved are ordered Smallest < Standard < Uncompressed, the size estimate is shown before and
the real size on the button after, and each saved file is opened again by the program: its TIC equals the Standard one (Uncompressed exactly, Smallest within the Numpress precision).
Needs the site and the .wasm: python tools/build_site.py (the .wasm is built by the CI in site/static/wasm)."""
import socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ or its .wasm not built: python tools/build_site.py")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="comp_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(300000):      # wait for the server to listen (connection attempts, not a fixed pause)
    try: socket.create_connection(("127.0.0.1", PORT), timeout=0.2).close(); break
    except OSError: pass
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
JSON = """async u => { const r = await fetch(u); return { status: r.status, j: await r.json() }; }"""

def fresh(b):
    c = b.new_context(viewport={"width": 1500, "height": 1800}, accept_downloads=True); c.set_default_timeout(120000); pg = c.new_page()
    pg.on("pageerror", lambda e: errs.append(("pageerror", str(e)))); pg.add_init_script(INIT)
    pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
    return c, pg

def tic_of(b, path):
    """The TIC of a saved mzML opened in a page of its own."""
    c, pg = fresh(b)
    try:
        pg.set_input_files("#pick", [str(path)])
        pg.wait_for_function("typeof E !== 'undefined' && E.files.length >= 1 && E.pending.length === 0", timeout=120000)      # one file opens by itself
        k = pg.evaluate("E.files.find(f => !f.gone).k")
        return pg.evaluate(JSON, f"api/chrom?k={k}&level=1")["j"]
    finally: c.close()

saved = {}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c, pg = fresh(b)
        pg.set_input_files("#pick", [str(D / "serie.wiff"), str(D / "serie.wiff.scan")])
        pg.wait_for_function("document.querySelectorAll('#flist [data-dl]').length >= 1", timeout=120000)
        pg.click("#samptab"); pg.wait_for_selector("#wtable:not([hidden])", timeout=60000)      # the landing shows the table of the samples only when asked
        def default_is_standard():
            pg.locator("#advconv summary").click()
            assert pg.eval_on_selector("#cvcomp", "e => e.value") == "standard"
            assert pg.locator("#cvcomp option").count() == 3
        step("the choice is there, three options, Standard selected", default_is_standard)
        def estimates():
            seen = {}
            for k in ("standard", "smallest", "none"):
                pg.select_option("#cvcomp", k); seen[k] = pg.inner_text("#cvest").strip()
            assert all(v.startswith("≈") for v in seen.values()), seen
            pg.select_option("#cvcomp", "standard")
        step("an estimated size is shown before", estimates)
        def save_all():
            for k in ("standard", "smallest", "none"):
                pg.select_option("#cvcomp", k)
                with pg.expect_download(timeout=120000) as d: pg.locator("#flist [data-dl]").first.click()
                path = D / f"serie_{k}.mzML"; d.value.save_as(str(path)); saved[k] = path
                pg.wait_for_function("document.querySelector('#flist [data-dl]').textContent.includes('(')", timeout=60000)
        step("the three mzML files are written and saved", save_all)
        def real_size_on_button():
            t = pg.inner_text("#flist [data-dl]")
            assert "(" in t and ("kB" in t or "MB" in t), t
        step("the real size shows on the download button after", real_size_on_button)
        def size_order():
            s = {k: v.stat().st_size for k, v in saved.items()}
            assert s["smallest"] < s["standard"] < s["none"], s
        step("sizes: Smallest < Standard < Uncompressed", size_order)
        c.close()
        def reread():
            std = tic_of(b, saved["standard"])
            raw = tic_of(b, saved["none"])
            if not (ROOT / "site" / "static" / "wasm-lettori" / "manifest.json").exists():      # the engine refuses Numpress arrays: the «aperti» module reads them
                print("   (Smallest re-read: skipped, build the modules: tools/build_lettori_wasm.sh site/static)"); sml = std
            else: sml = tic_of(b, saved["smallest"])
            assert len(std["rt"]) == len(raw["rt"]) == len(sml["rt"]) == 80, (len(std["rt"]), len(raw["rt"]), len(sml["rt"]))
            assert std["y"] == raw["y"], "uncompressed differs from standard"
            worst = max(abs(a - b_) / max(abs(a), 1e-9) for a, b_ in zip(std["y"], sml["y"]) if abs(a) > 0)
            assert worst < 1e-3, worst          # Numpress: about 0.02 % in intensity
        step("each saved file opens again; Uncompressed equals Standard, Smallest within the Numpress precision", reread)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
