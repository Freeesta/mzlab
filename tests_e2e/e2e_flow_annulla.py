"""Flow undo from the keyboard: Ctrl+Z / Ctrl+Y reach the plan while the focus is on a select of the form (the key is not swallowed as a text-field undo).
Run from the repo root: python3 tests_e2e/e2e_flow_annulla.py"""
import socket, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp()); site = tmp / "site"
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.click('#nav button[data-v="flow"]'); pg.wait_for_selector("#flx-root", timeout=30000)
        pg.click('[data-tab="list"]'); pg.click('button[aria-label="Start a new plan"]'); pg.evaluate("document.querySelector('.flx-pal').remove()")
        for s in ["convert.mzml", "index.mzidx", "features.detect", "features.align"]: pg.click(f'.fl-it[data-step="{s}"]')
        attendi(pg, "document.querySelectorAll('.fl-card').length === 4")
        def undo_on_a_select():
            val = lambda: pg.evaluate("document.querySelector('#fl-alg-detect').value")
            before = val()
            other = pg.evaluate("[...document.querySelectorAll('#fl-alg-detect option')].map(o => o.value).find(v => v && v !== document.querySelector('#fl-alg-detect').value && !/threshold/.test(v))")
            assert other, "no second available algorithm"
            pg.select_option("#fl-alg-detect", other); attendi(pg, f"document.querySelector('#fl-alg-detect').value === '{other}'")
            pg.focus("#fl-alg-detect"); pg.keyboard.press("Control+z"); attendi(pg, f"document.querySelector('#fl-alg-detect').value === '{before}'")
            pg.focus("#fl-alg-detect"); pg.keyboard.press("Control+y"); attendi(pg, f"document.querySelector('#fl-alg-detect').value === '{other}'")
        step("Ctrl+Z and Ctrl+Y work with the focus on a select of the form", undo_on_a_select)
        b.close()
finally:
    srv.terminate()
print("STEPS"); [print(" ", s) for s in steps]; print("page errors:", errs)
sys.exit(1 if errs or any(s[1] != "ok" for s in steps) else 0)
