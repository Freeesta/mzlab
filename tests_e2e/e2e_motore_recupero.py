"""Engine start recovery (issue #3509; browser.js engineCacheSuspect, engine-recupera.js, reset.js mode "engine"), on the built site with its real engine.
A server that can answer a path with other bytes stands for a stored copy of the site that mixes two deploys (a worker that imports names its .wasm glue lacks).
1 mixed cache: the engine dies at start, the page drops the stored copy through the reset flow, reloads ONCE (ripn nonce) and the engine starts; the flag is cleared;
2 the same fault on the network too: ONE recovery reload, then the banner «load.err.engineStartPurged» (no loop);
3 a .wasm that does not compile (not a cache matter): no recovery reload, the page ends in its own start error.
Build the site first: python3 tools/prepara_sito.py. Run from the repo root: python3 tests_e2e/e2e_motore_recupero.py"""
import os, re, sys, threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, urlparse
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402
from lib import free_port, wait_for_js  # noqa: E402

SITE = Path(os.environ.get("MZLAB_PIPE_SITE") or ROOT / "site")
BIG = "mzlab-big-" + re.search(r'CACHE_BIG : "mzlab-big-"\) \+ "([\w-]+)"', (SITE / "sw.js").read_text()).group(1)       # the worker keeps only its own big cache: the probe is named like it
GLUE, WASM = "/static/wasm/mzlab_wasm.js", "/static/wasm/mzlab_wasm_bg.wasm"
OLD_GLUE = b"export default function init() { return Promise.resolve({}); }\n"      # no named exports: rust-worker.js fails to link, the worker dies at start
MIME = {"js": "text/javascript", "html": "text/html", "css": "text/css", "json": "application/json", "wasm": "application/wasm"}
over: dict = {}                                            # path -> bytes the server answers instead of the file


class Pages(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"
    def log_message(self, *a): pass
    def do_GET(self):
        p = unquote(urlparse(self.path).path)
        f = SITE / (p.lstrip("/") or "index.html")
        if f.is_dir(): f = f / "index.html"
        data = over.get(p) if p in over else (f.read_bytes() if f.is_file() else None)
        if data is None:
            self.send_response(404); self.send_header("Content-Length", "0"); self.end_headers(); return
        self.send_response(200); self.send_header("Content-Type", MIME.get(f.suffix[1:], "application/octet-stream"))
        self.send_header("Cache-Control", "no-cache"); self.send_header("Content-Length", str(len(data))); self.end_headers()
        try: self.wfile.write(data)
        except ConnectionError: pass


class Srv(ThreadingHTTPServer):
    request_queue_size = 256; daemon_threads = True
    def handle_error(self, *a): pass


errs: list = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + repr(e)[-700:].replace("\\n", " "))); errs.append(name)


if not (SITE / "static" / "wasm" / "mzlab_wasm_bg.wasm").is_file(): raise SystemExit("build the site with its engine first: python3 tools/prepara_sito.py")
port = free_port(); srv = Srv(("127.0.0.1", port), Pages); threading.Thread(target=srv.serve_forever, daemon=True).start()
URL = f"http://127.0.0.1:{port}/"
SKIP = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"


def session(p):
    """A fresh browser context: counts the recovery logs and the page loads that carry the recovery nonce."""
    b = p.chromium.launch(); ctx = b.new_context(); ctx.add_init_script(SKIP); pg = ctx.new_page()
    seen = {"recover": 0, "ripn": 0}
    pg.on("console", lambda m: seen.__setitem__("recover", seen["recover"] + ("engine did not start: dropping" in m.text)))
    pg.on("framenavigated", lambda f: seen.__setitem__("ripn", seen["ripn"] + (f == pg.main_frame and "ripn=" in f.url)))
    return b, pg, seen


try:
    with sync_playwright() as p:
        def mixed_cache():
            over.clear(); b, pg, seen = session(p)
            pg.goto(URL); pg.wait_for_function("navigator.serviceWorker.controller !== null", timeout=60000)
            wait_for_js(pg, "!!window.MZLAB_LR", 120000)                                 # a healthy first visit: the engine starts
            wait_for_js(pg, """(async () => { const n = (await caches.keys()).find(k => k.startsWith('mzlab-app-')); return !!n && !!(await (await caches.open(n)).match('.fill-done')); })()""", 150000)
            pg.evaluate("""async glue => { const n = (await caches.keys()).find(k => k.startsWith('mzlab-app-')), c = await caches.open(n);
                await c.put(new URL(glue, location.origin).href, new Response('export default function init() { return Promise.resolve({}); }\\n', { headers: { 'Content-Type': 'text/javascript' } })); }""", GLUE)
            pg.evaluate("sessionStorage.removeItem('mzlab.engineRecover')")
            pg.evaluate("n => caches.open(n).then(c => c.put('/probe', new Response('x')))", BIG)      # a vendor copy: the "engine" mode must leave it
            seen.update(recover=0, ripn=0)
            pg.reload()                                                                  # the stored copy now mixes two versions: the engine dies, the page recovers
            wait_for_js(pg, "!!window.MZLAB_LR", 180000)
            assert seen["recover"] == 1, ("recovery logged", seen)
            assert seen["ripn"] == 2, ("one recovery load (ripn) and the single isolating reload after it", seen)
            assert pg.evaluate("caches.keys()").count(BIG) == 1, "the big caches survive the engine mode"
            assert pg.evaluate("sessionStorage.getItem('mzlab.engineRecover')") is None, "the flag is cleared once the engine is up"
            assert pg.evaluate("window.qqFailed") is None, pg.evaluate("window.qqFailed")
            b.close()
        step("mixed-version cache: the engine starts after ONE recovery reload", mixed_cache)

        def purged_banner():
            over.clear(); over[GLUE] = OLD_GLUE; b, pg, seen = session(p)               # the network serves the faulty copy too: the purge cannot help
            pg.goto(URL); wait_for_js(pg, "window.qqFailed === 'load.err.engineStartPurged'", 180000)
            assert seen["recover"] == 1 and seen["ripn"] == 2, ("one recovery (+ the isolating reload), no loop", seen)
            assert "still does not start" in pg.evaluate("document.body.innerText"), "the banner text is on the page"
            assert pg.evaluate("!!window.MZLAB_LR") is False
            b.close()
        step("still dead after the recovery: «load.err.engineStartPurged», one reload, no loop", purged_banner)

        def broken_wasm():
            over.clear(); over[WASM] = b"not a wasm module"; b, pg, seen = session(p)  # the .wasm itself does not compile: clearing the stored copy cannot help
            pg.goto(URL + "?ripresa=3000"); wait_for_js(pg, "!!window.qqFailed", 120000)      # the worker does not die on it: the start ends at the reopening limit
            assert seen["recover"] == 0 and seen["ripn"] == 0, ("no recovery reload", seen)
            assert pg.evaluate("window.qqFailed") != "load.err.engineStartPurged", "no purge banner"
            b.close()
        step("a .wasm that does not compile: no recovery reload", broken_wasm)
finally:
    srv.shutdown()
if errs:
    print("FAIL:", errs); sys.exit(1)
print("PASS e2e_motore_recupero")
