"""Warning «this phase needs more memory» (memoria-avviso.js, asked by feat-run.js through window.mzlabAskMore): two buttons, the safe one («Cancel») is the default (Enter, Esc),
«Use more memory» resolves true; the text carries the numbers and the phase in the language of the page.
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
INFO = '{ fase: "align", stima_mb: 1400, budget_mb: 512, libero_mb: 300, high_mb: 3072 }'
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        ctx = b.new_context(viewport={"width": 1300, "height": 900}, locale="en-US")
        pg = ctx.new_page(); errs = []
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/?lang=en")
        pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.mzlabAskMore === 'function'", timeout=60000)
        def ask(): pg.evaluate(f"window.__r = undefined; window.mzlabAskMore({INFO}).then(v => window.__r = v)"); pg.wait_for_selector("#memmore")
        def text():
            t = pg.inner_text("#memmore")
            for s in ("1400", "512", "3072", "300", "Use more memory", "Cancel"): assert s in t, (s, t)
            assert pg.evaluate("document.activeElement.id") == "mm-no", "the safe button has the focus"
            assert pg.get_attribute("#memmore .mm-box", "role") == "alertdialog"
        ask(); step("the text has the numbers, the focus is on the safe button", text)
        def enter(): pg.keyboard.press("Enter"); pg.wait_for_function("window.__r === false"); assert pg.query_selector("#memmore") is None
        step("Enter = Cancel: false, the dialog closes", enter)
        def yes(): ask(); pg.click("#mm-yes"); pg.wait_for_function("window.__r === true"); assert pg.query_selector("#memmore") is None
        step("«Use more memory»: true", yes)
        def esc(): ask(); pg.keyboard.press("Escape"); pg.wait_for_function("window.__r === false")
        step("Esc = Cancel: false", esc)
        def tab(): ask(); pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.id") == "mm-yes"; pg.keyboard.press("Tab"); assert pg.evaluate("document.activeElement.id") == "mm-no"; pg.keyboard.press("Escape")
        step("Tab stays inside the dialog", tab)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
