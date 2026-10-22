"""Flow, the «Import data» card (docs/agenti/flow-grafica.md): it says what the Data tab holds (no file / N files), shows no stray «null» and no «Ready.».
Run from the repo root: python3 tests_e2e/e2e_flow_import.py"""
import subprocess, sys, tempfile, socket
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
site = Path(tempfile.mkdtemp()) / "site"
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
bad = []
def check(name, ok, info=""):
    if not ok: bad.append(f"{name}: {info}")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1400, "height": 1000}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.click('#nav button[data-v="flow"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
        pg.click('.fln:has-text("Import data")'); attendi(pg, "document.querySelector('.fl-insp h3') && document.querySelector('.fl-insp h3').textContent === 'Import data'")
        t = pg.inner_text(".fl-insp")
        check("no file", "No file loaded" in t and pg.locator(".fl-insp button:has-text('Open Data')").count() == 1, t)
        check("no filler", "null" not in t.split() and "Ready." not in t and "What this step does" not in t and "Feeds into" not in t, t)
        pg.evaluate("E.files.push({file: 'a.mzML', name: 'a'}, {file: 'b.mzML', name: 'b'})")
        pg.click('.fln:has-text("Build feature table")'); pg.click('.fln:has-text("Import data")')
        attendi(pg, "!!document.querySelector('.fl-insp .fl-data') && /in Data/.test(document.querySelector('.fl-insp .fl-data').textContent)")
        t = pg.inner_text(".fl-insp")
        check("files", "2 files in Data: a.mzML, b.mzML" in t and "No file loaded" not in t, t)
        b.close()
finally:
    srv.terminate()
print("FAIL\n" + "\n".join(bad) if bad else "OK e2e_flow_import")
sys.exit(1 if bad else 0)
