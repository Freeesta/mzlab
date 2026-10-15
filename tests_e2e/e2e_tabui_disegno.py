"""Tabui, «Draw» of the parent (TP_Mine/js/20_tpmine.js, mzlab/web/draw.js TPDraw.when/load): the editor loads on demand, so «Use this structure» stays off with «Loading the editor...»
until it is ready (never «Nothing drawn yet.» for an editor that has not started); a structure drawn in the editor and brought back with «Use this structure» fills the SMILES and the
formula and returns to Tabui; the other way, a SMILES already written in Tabui is shown in the editor by «Draw». With nothing drawn the message says so.
Run from the repo root: python3 tests_e2e/e2e_tabui_disegno.py"""
import os, shutil, subprocess, sys, tempfile, time, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
reuse = os.environ.get("MZLAB_PIPE_SITE")
if reuse: site = Path(reuse)
else:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):                                                  # the server answers: no fixed pause
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: __import__("threading").Event().wait(0.1)
errs = []
TAB = '#nav button[data-v="tpmine"]'
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh():
            c = b.new_context(viewport={"width": 1400, "height": 1000}, reduced_motion="reduce"); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/?lang=en"); pg.wait_for_selector(TAB, timeout=60000)
            pg.click(TAB); pg.wait_for_selector("#tp-draw")
            return pg
        KET = "document.querySelector('#kframe').contentWindow.ketcher"
        pg = fresh()
        def loading():
            pg.click("#tp-draw")
            assert pg.is_disabled("#tp-draw-use") or pg.evaluate("TPDraw.ready()"), "«Use» must wait for the editor"
            assert "Nothing drawn" not in pg.inner_text("body")
            pg.wait_for_function("TPDraw.ready() && !document.querySelector('#tp-draw-use').disabled", timeout=90000)
            assert pg.inner_text("#tp-draw-st") == ""
        step("«Draw» waits for the editor: «Use» is off while it loads, then on", loading)
        def empty():
            pg.click("#tp-draw-use"); pg.wait_for_function("document.body.innerText.includes('Nothing drawn yet.')", timeout=10000)
            assert pg.evaluate("S.view") == "draw"
        step("nothing drawn: the message says so and the Draw tab stays", empty)
        def drawn():
            pg.evaluate(f"{KET}.setMolecule('CCO')")
            pg.wait_for_function("TPDraw.smiles().then(s => s === 'CCO')", timeout=10000)
            pg.click("#tp-draw-use")
            pg.wait_for_function("document.querySelector('#tp-smi').value === 'CCO'", timeout=10000)
            pg.wait_for_function("document.querySelector('#tp-mol').value === 'C2H6O'", timeout=30000)
            assert pg.evaluate("S.view") == "tpmine" and pg.evaluate("document.getElementById('tp-drawbar').hidden")
        step("a structure drawn comes back as SMILES and formula", drawn)
        def reverse():
            pg.fill("#tp-smi", "CCN"); pg.click("#tp-draw")
            pg.wait_for_function("TPDraw.smiles().then(s => s === 'CCN')", timeout=30000)
            assert not pg.is_disabled("#tp-draw-use")
        step("a SMILES written in Tabui is shown in the editor by «Draw»", reverse)
        pg.click("#tp-draw-no"); assert pg.evaluate("S.view") == "tpmine"
        b.close()
finally:
    srv.terminate()
    if not reuse: shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
