"""Clicks and keys of the flagship flow of Tabui (study WORKFLOW_MODULARI, test 4): from the files to the TP table, on the synthetic HR series of tests/ (lc_synth, msn_synth), counted by
capture listeners in the page (trusted click, key chord, no modifier alone). The OS file dialog (click on «Choose files...», Ctrl+A, Enter) and the paste of the SMILES (click on the field, Ctrl+V)
cannot be driven by Playwright, so they are added as declared constants; opening Tabui (5 clicks on the logo, once per browser) is reported apart. The line «MISURA» goes in docs/agenti/misure-clic.md.
Run from the repo root: python3 tests_e2e/e2e_clic_flusso.py"""
import subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port; PORT = free_port()
CEILING = 10         # clicks + keys without the logo; a flow that needs more is a regression (rule 26 of the study: few clicks)
DIALOG = {"click": 1, "key": 2}      # «Choose files...», then Ctrl+A and Enter in the dialog of the system
PASTE = {"click": 1, "key": 1}       # the field of the SMILES, then Ctrl+V
ADMIN = 5                            # clicks on the logo that show Tabui, once per browser
COUNT = """() => { window.__n = {click: 0, key: 0}; const M = ['Shift', 'Control', 'Alt', 'Meta'];
  addEventListener('click', e => { if (e.isTrusted) window.__n.click++; }, true);
  addEventListener('keydown', e => { if (e.isTrusted && !M.includes(e.key) && !e.repeat) window.__n.key++; }, true); }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600] + " | " + str(pg.evaluate("[(document.querySelector('#tp-msg') || {}).innerText, document.querySelector('#tp-files').innerHTML.slice(0, 200), (document.querySelector('#tp-in') || {}).files && document.querySelector('#tp-in').files.length]"))))
tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
from lib import build_feat; build_feat(tmp / "site" / "static", dict(__import__("os").environ, RUNNER_TEMP=str(tmp / "rt")))      # the engine of mzFinder
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):                 # the server answers once the port is open (a refused connection returns at once)
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}, reduced_motion="reduce").new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.evaluate(COUNT)
        def flow():
            sys.path[:0] = [str(ROOT / "TP_Mine" / "tests"), str(ROOT)]
            import lc_synth, msn_synth
            d = Path(tempfile.mkdtemp()); fs = lc_synth.write_series(d, hidden_isf=True); mf = msn_synth.write_msn(d / "cafe_msn.mzML", msn_synth.caffeine_nodes())
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.set_input_files("#tp-in", [f["path"] for f in fs] + [str(mf)])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 8", timeout=300000)
            pg.click("#tp-smi"); pg.keyboard.insert_text("Cn1cnc2c1c(=O)n(C)c(=O)n2C")
            pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('195.0877')", timeout=60000)
            pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            n = pg.evaluate("window.__n")
            # the field of the SMILES was clicked above: its click is the one of PASTE, so it is not counted twice
            clicks = n["click"] - 1 + DIALOG["click"] + PASTE["click"]; keys = n["key"] + DIALOG["key"] + PASTE["key"]
            print(f"MISURA clic_flusso: tab 1 + files {DIALOG['click']}c/{DIALOG['key']}k + SMILES {PASTE['click']}c/{PASTE['key']}k + Dig 1 = {clicks} clicks + {keys} keys = {clicks + keys}; with the {ADMIN} clicks of the logo {clicks + keys + ADMIN}")
            assert clicks + keys <= CEILING, (clicks, keys)
        step("files to the TP table in few clicks and keys", flow)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
