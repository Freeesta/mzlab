"""Flow, a saved plan at an older step version (table.filter@1) opened WITHOUT the Terminal panel (flow/lazy/flow-terminal.js not needed): the engine migrates it (route api/flow/line,
mzlab_core::flow::migrate), the page does not refuse it and shows the current version, as `mzlab-cli flow run` does. Run from the repo root: python3 tests_e2e/e2e_flow_migrate.py"""
import json, subprocess, sys, socket
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
subprocess.run([sys.executable, str(ROOT / "tools" / "prepara_sito.py")], check=True)
site = ROOT / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
bad = []
def check(name, ok, info=""):
    if not ok: bad.append(f"{name}: {info}")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1400, "height": 1000}); ctx.grant_permissions(["clipboard-read", "clipboard-write"], origin=f"http://127.0.0.1:{PORT}")
        pg = ctx.new_page(); pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.click('#nav button[data-v="flow"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
        old = json.loads((ROOT / "crates/mzlab-cli/tests/data/flow_migrate_case.json").read_text(encoding="utf-8"))["old"]
        pg.set_input_files("#fl-file", files=[{"name": "old.mzflow.json", "mimeType": "application/json", "buffer": json.dumps(old).encode()}])
        attendi(pg, "/Opened old.mzflow.json/.test(document.body.textContent)")
        steps = pg.evaluate("FLOW.current.plan.steps.map(s => s.step)")
        check("migrated to the current version", steps == ["table.filter@2"], steps)
        check("no version problem", "has version" not in pg.inner_text("#fl-root"), pg.inner_text("#fl-root")[:300])
        b.close()
finally:
    srv.terminate()
if bad: print("FAIL:\n" + "\n".join(bad)); sys.exit(1)
print("OK: a plan with table.filter@1 opens migrated, as the CLI runs it")
