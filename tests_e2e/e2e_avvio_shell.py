"""The Dati page draws its shell before the engine: with the .wasm held back, the drop area is visible and usable-looking while the engine is still
pending, the .wasm is asked for only once the page is parsed (not from the head, where it competes with the first paint), and once released
the engine comes up on its own. Build first: python tools/build_site.py"""
from pathlib import Path
import subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

PORT = free_port()
errs, page_errs = [], []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)

# the state of the document when the page asks for the .wasm (the fetch of the page goes through this wrapper: browser.js keeps it as its real fetch)
WATCH = """(() => {
  window.__wasmAsked = []; const real = window.fetch.bind(window);
  window.fetch = (input, init) => { const u = String(input && input.url || input); if (/\\.wasm(\\?|$)/.test(u)) window.__wasmAsked.push(document.readyState); return real(input, init); };
})()"""

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
URL = f"http://127.0.0.1:{PORT}/"
held, head_held, asked_urls = [], [], []
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1400, "height": 900}, reduced_motion="reduce")
        ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');}catch(_){}")
        ctx.add_init_script(WATCH)
        ctx.route("**/*.wasm", lambda r: held.append(r))
        ctx.route("**/static/version.js", lambda r: head_held.append(r))     # a script of the head: the parser waits for it
        ctx.on("request", lambda q: asked_urls.append(q.url))
        pg = ctx.new_page(); pg.on("pageerror", lambda e: page_errs.append(str(e)))
        pg.goto(URL, wait_until="commit")

        def engine_waits_for_the_parser():
            pg.wait_for_timeout(1500)
            assert head_held, "the head script was not asked for"
            assert not held and not any("rust-bridge.js" in u for u in asked_urls), "the engine started while the head was still being read"
            for r in head_held: r.continue_()
        step("the engine does not start while the head is being read", engine_waits_for_the_parser)

        def shell_before_engine():
            pg.wait_for_selector("#start", state="visible", timeout=30000)
            assert pg.is_visible("#drop") or pg.is_visible("#drop-skeleton"), "no drop area in the shell"
            deadline = time.time() + 30
            while not held and time.time() < deadline: pg.wait_for_timeout(50)
            assert held, "the engine never asked for the .wasm"
            assert pg.evaluate("typeof AVVIO !== 'undefined' && AVVIO.stato") == "pending", "the engine was ready with the .wasm held back"
            assert pg.is_visible("#start"), "the shell went away while the engine was pending"
        step("shell visible while the .wasm is held back", shell_before_engine)

        def wasm_after_parse():
            asked = pg.evaluate("window.__wasmAsked")
            assert asked and all(s != "loading" for s in asked), f"the .wasm was asked for while the page was being parsed: {asked}"
        step(".wasm asked for only after the page is parsed", wasm_after_parse)

        def engine_follows():
            for r in held: r.continue_()
            deadline = time.time() + 90
            while pg.evaluate("typeof AVVIO !== 'undefined' && AVVIO.stato") != "ready" and time.time() < deadline: pg.wait_for_timeout(100)
            assert pg.evaluate("AVVIO.stato") == "ready", "the engine did not come up after the .wasm was released"
            assert pg.is_visible("#start")
        step("released, the engine comes up on its own", engine_follows)
        b.close()
finally:
    srv.terminate()
if page_errs: print("page errors:", page_errs); errs.append("page errors")
sys.exit(1 if errs else 0)
