"""The tab title of the browser has one format on every page: «<Page> · mzLab» (page name first, separator «·»): the landing and Dati («MS Analysis» / «Analisi MS»),
Draw, Flow and the Tabui tabs. Run from the repo root: python3 tests_e2e/e2e_titolo_scheda.py"""  # covers: mzlab/web/shell.js, mzlab/web/avvio.js, mzlab/web/lang/
import subprocess, sys, tempfile, re
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, wait_for_js; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))
FORMAT = re.compile(r"^[^·•]+ · mzLab$")
tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        for loc, tag in (("it-IT", "Analisi MS"), ("en-US", "MS Analysis")):
            c = b.new_context(viewport={"width": 1400, "height": 900}, locale=loc); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000)
            def landing(tag=tag): wait_for_js(pg, f"document.title === '{tag} · mzLab'")
            step(f"{loc}: landing/Dati title is «{tag} · mzLab»", landing)
            views = pg.evaluate("[...document.querySelectorAll('#nav button[data-v]')].map(b => b.dataset.v)")
            def each(views=views, loc=loc):
                assert {"data", "draw", "flow"} <= set(views), views
                for v in views:
                    pg.click(f'#nav button[data-v="{v}"]'); wait_for_js(pg, f"S.view === '{v}'")
                    t = pg.title(); assert FORMAT.match(t), (v, t)
                    if v == "data": assert t == f"{tag} · mzLab", t
            step(f"{loc}: every page has the title «<Page> · mzLab»", each)
            c.close()
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
