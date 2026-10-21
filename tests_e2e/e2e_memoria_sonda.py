"""Growth probe and device facts of the memory budget (rust-sonda.js, rust-budget.js): the probe runs once, its result reaches the register of the
choices (inputs.probe_mb), and Rust's table gives safe/high levels that fit the declared device memory (deviceMemory forced to 2, 4 and 8).
Needs the site: python tools/build_site.py and the .wasm (tools/prepara_sito.py)."""
from pathlib import Path
import socket, subprocess, sys, threading
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:400]))

if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):          # the server is up when the port answers
    try: socket.create_connection(("127.0.0.1", PORT), timeout=0.2).close(); break
    except OSError: threading.Event().wait(0.1)
EXPECT = {2: (128, 192), 4: (512, 1024), 8: (1024, 3072)}      # safe, high for a computer with fewer than 8 cores (the table of mzlab-core::budget_auto)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        for gb, (safe, high) in EXPECT.items():
            ctx = b.new_context(viewport={"width": 1300, "height": 900})
            ctx.add_init_script(f"Object.defineProperty(Navigator.prototype, 'deviceMemory', {{ get: () => {gb} }}); Object.defineProperty(Navigator.prototype, 'hardwareConcurrency', {{ get: () => 4 }});")
            pg = ctx.new_page(); errs = []
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.goto(f"http://127.0.0.1:{PORT}/")
            def choice():
                pg.wait_for_selector("#drop", timeout=120000)
                pg.wait_for_function("window.MZLAB_RUST && window.MZLAB_RUST.choice", timeout=60000)
                c = pg.evaluate("window.MZLAB_RUST.choice")
                assert (c["safe_mb"], c["high_mb"]) == (safe, high) or c["inputs"]["probe_mb"] < high, (gb, c)      # the probe may only lower the high level
                assert c["safe_mb"] <= c["high_mb"] and c["mb"] == c["safe_mb"], c
                assert c["inputs"]["device_gb"] == gb and c["inputs"]["probe_mb"] > 0, c["inputs"]
            step(f"deviceMemory {gb}: safe/high levels from Rust, probe in the inputs", choice)
            def probe_kept():
                p = pg.evaluate("JSON.parse(sessionStorage.getItem('mzlab.memProbe') || 'null')")
                assert p and p["mb"] > 0 and p["ms"] < 200, p
                log = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.memBudget.log') || '[]')")
                assert log and "probe_ms" in log[-1]["inputs"], log
            step(f"deviceMemory {gb}: the probe is kept for the session, takes under 200 ms and is in the log", probe_kept)
            step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
            ctx.close()
        b.close()
finally:
    srv.terminate()
