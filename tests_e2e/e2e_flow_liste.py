"""Flow, a parameter that is a list of the user (docs/agenti/flow-grafica.md): the adducts of «Inclusion list» are edited in a table inside the inspector (add and remove rows, import a CSV,
export JSON, SHA-256 shown, no dialog); every change reaches the plan, whose text (and so its SHA-256) changes with the list. The app is faked at the native route flow/liste (flow_desktop_fake.js).
Run from the repo root: python3 tests_e2e/e2e_flow_liste.py"""
import hashlib, json, subprocess, sys, tempfile, socket
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
site = tmp / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []
TAB = '#nav button[data-v="flow"]'; FAKE = "".join(l for l in (HERE / "flow_desktop_fake.js").open() if not l.startswith("//"))
PLAN = "document.querySelector('.flx-list').__flow.plan"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); c = b.new_context(viewport={"width": 1400, "height": 1000}, accept_downloads=True); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("dialog", lambda d: errs.append("dialog: " + d.message))
        pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector(TAB, timeout=120000); pg.evaluate(FAKE)
        pg.click(TAB); pg.wait_for_selector("#flx-root", timeout=30000)
        pg.click('[data-tab="list"]'); pg.click('button[aria-label="Start a new plan"]'); pg.evaluate("document.querySelector('.flx-pal').remove()")
        pg.click('.fl-it[data-step="export.inclusion"]')
        text = lambda: pg.evaluate(f"FLOW.exportText({PLAN})")
        sha = lambda: hashlib.sha256(text().encode()).hexdigest()
        def start():
            attendi(pg, "!!document.querySelector('.fl-card button[aria-label^=\"Use my own list\"]')")
            before = sha(); pg.click('.fl-card button[aria-label^="Use my own list"]')
            attendi(pg, "document.querySelectorAll('.fl-list tbody tr').length === 1 && !!document.querySelector('.fl-list-sha').dataset.sha")
            assert sha() != before and "name;delta;charge;comment" in text(), text()
        step("«Use my own list» turns the names into a one-row table and changes the plan", start)
        def edit():
            first = pg.evaluate("document.querySelector('.fl-list-sha').dataset.sha"); plan = sha()
            pg.click('.fl-list button[aria-label^="Add a row"]')
            pg.fill('.fl-list tbody tr:nth-child(2) input[data-col=name]', "[M+Na]+"); pg.fill('.fl-list tbody tr:nth-child(2) input[data-col=delta]', "+Na")
            pg.locator('.fl-list tbody tr:nth-child(2) input[data-col=delta]').dispatch_event("change")
            attendi(pg, "document.querySelectorAll('.fl-list tbody tr').length === 2 && document.querySelector('.fl-list-sha').dataset.sha !== '%s'" % first)
            assert sha() != plan and "[M+Na]+" in text()
            pg.fill('.fl-list tbody tr:nth-child(2) input[data-col=name]', ""); pg.locator('.fl-list tbody tr:nth-child(2) input[data-col=name]').dispatch_event("change")
            attendi(pg, "document.querySelector('.fl-list-err li') && document.querySelector('.fl-list-err').textContent.includes('noName')")
            assert "[M+Na]+" in text(), "a wrong row must not reach the plan"
            pg.click('.fl-list tbody tr:nth-child(2) button[aria-label^="Remove row"]')
            attendi(pg, "document.querySelectorAll('.fl-list tbody tr').length === 1 && document.querySelector('.fl-list-sha').dataset.sha === '%s'" % first)
        step("add a row, a wrong row is refused with its error, remove it: the SHA-256 follows the list", edit)
        def imp():
            plan = sha()
            pg.set_input_files('.fl-list input[type=file]', files=[{"name": "my.csv", "mimeType": "text/csv", "buffer": b"name;delta;charge;comment\n[M+K]+;+K;1;potassium\n[M+NH4]+;+NH4;1;\n"}])
            attendi(pg, "document.querySelectorAll('.fl-list tbody tr').length === 2")
            assert pg.input_value('.fl-list tbody tr:nth-child(1) input[data-col=name]') == "[M+K]+" and sha() != plan and "potassium" in text()
        step("importing a CSV replaces the rows and the plan follows", imp)
        def export():
            with pg.expect_download() as d: pg.click('.fl-list button[aria-label^="Export"]')
            doc = json.loads(Path(d.value.path()).read_text()); assert doc["schema"] == "liste/1" and len(doc["elementi"]) == 2, doc
        step("export gives the list as JSON", export)
        def small():
            r = pg.evaluate("[...document.querySelectorAll('.fl-list button, .fl-list input:not([hidden])')].map(e => e.getBoundingClientRect()).filter(x => x.width > 0).every(x => x.width >= 24 && x.height >= 24)")
            assert r, "targets under 24 px"
            pg.focus('.fl-list tbody tr:nth-child(1) input[data-col=name]'); pg.keyboard.press("Tab")
            assert pg.evaluate("document.activeElement.dataset.col") == "delta"
        step("targets are at least 24 px and the cells follow the keyboard", small)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
