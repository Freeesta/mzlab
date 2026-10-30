"""Service worker cache-first by site version (mzlab/web/sw.js, tools/build_site.py seal_sw): three builds of the site served one after the other
to the same browser, with the headers of GitHub Pages (ETag, max-age=600). Build the site first: python tools/build_site.py.
1 a normal reload after a deploy shows the new version at once (static/version.js) and the page never mixes files of two builds;
2 the old cache goes, the new one is filled, and the next reload asks the server for no static file at all (cache-first);
3 «Full reset» (QQRESET.wipe, then the reload it does) brings the newest version although the HTTP cache holds the old files."""
import hashlib, re, shutil, socket, subprocess, sys, tempfile, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, urlparse
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port

def serve(port: int, wd: Path):
    class Pages(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"
        def log_message(self, *a): pass
        def do_GET(self):
            p = unquote(urlparse(self.path).path)
            root = Path((wd / "root.txt").read_text())
            f = root / (p.lstrip("/") or "index.html")
            if f.is_dir(): f = f / "index.html"
            if not f.is_file():
                self.send_response(404); self.send_header("Content-Length", "0"); self.end_headers(); return
            with open(wd / "hits.log", "a") as h: h.write(p + "\n")
            data = f.read_bytes(); etag = '"' + hashlib.md5(data).hexdigest() + '"'
            if self.headers.get("If-None-Match") == etag:
                self.send_response(304); self.send_header("ETag", etag); self.send_header("Content-Length", "0"); self.end_headers(); return
            ct = {"js": "text/javascript", "html": "text/html", "css": "text/css", "json": "application/json", "wasm": "application/wasm"}.get(f.suffix[1:], "application/octet-stream")
            self.send_response(200); self.send_header("Content-Type", ct); self.send_header("ETag", etag); self.send_header("Cache-Control", "max-age=600")
            self.send_header("Content-Length", str(len(data))); self.end_headers()
            try: self.wfile.write(data)
            except ConnectionError: pass
    class Srv(ThreadingHTTPServer):
        request_queue_size = 256; daemon_threads = True
        def handle_error(self, *a): pass
    Srv(("127.0.0.1", port), Pages).serve_forever()


if len(sys.argv) > 1 and sys.argv[1] == "--serve":
    serve(int(sys.argv[2]), Path(sys.argv[3])); sys.exit(0)


SITE = ROOT / "site"
work = Path(tempfile.mkdtemp(prefix="sw_versione_"))
state: dict = {}


def build(name: str, version: str):
    """A copy of the built site whose static/version.js and index.html differ, sealed again (new cache name, new sw-manifest)."""
    d = work / name
    shutil.copytree(SITE, d)
    (d / "static" / "version.js").write_text(f'window.MZLAB_VERSION = "{version}";\nwindow.MZLAB_COMMIT = "{version}";\n', encoding="utf-8")
    (d / "index.html").write_text((d / "index.html").read_text(encoding="utf-8") + f"<!-- {version} -->", encoding="utf-8")
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--build-json", "--out", str(d)], check=True, stdout=subprocess.DEVNULL)


class Hits:
    """Requests the server process has answered since the last clear(), counted by path (the server writes them to a log file)."""
    def __init__(self, path: Path): self.path = path
    def clear(self): self.path.write_text("")
    def get(self, k, d=0): return self._all().get(k, d)
    def items(self):
        return self._all().items()
    def _all(self):
        out = {}
        for ln in self.path.read_text().split("\n"):
            if ln: out[ln] = out.get(ln, 0) + 1
        return out


class Root:
    """Which build the server process serves (a file it reads on every request)."""
    def __init__(self, path: Path): self.path = path
    def __setitem__(self, k, v): self.path.write_text(str(v))


errs = []
V0: list = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + repr(e)[-700:].replace("\\n", " "))); errs.append(name)


def static_hits(): return {k: v for k, v in state["hits"].items() if k.startswith("/static/") and not k.endswith(("manifest.json", "build.json"))}


def wait_cache(pg, app, timeout=150):
    """Waits for the background fill of the cache `app`; a page request now and then wakes a worker the browser stopped (the fill resumes where it was)."""
    t0 = time.time()
    while time.time() - t0 < timeout:
        if pg.evaluate("""async app => {
            for (let i = 0; i < 6; i++) {
                const k = await caches.keys();
                if (k.includes('mzlab-app-' + app) && await (await caches.open('mzlab-app-' + app)).match('.fill-done')) return true;
                await new Promise(r => setTimeout(r, 500));
            }
            fetch('static/version.js').then(r => r.text());
            return false; }""", app): return
    raise AssertionError("the cache of " + app + " was not filled in time")


SKIP = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
try:
    if not (SITE / "sw.js").is_file(): raise SystemExit("build the site first: python tools/build_site.py")
    shutil.copytree(SITE, work / "n0")
    for n, v in (("n1", "N1"), ("n2", "N2")): build(n, v)
    app = {n: re.search(r'APPV = "(\w+)"', (work / n / "sw.js").read_text()).group(1) for n in ("n0", "n1", "n2")}
    assert len(set(app.values())) == 3, app
    port = free_port()
    state["root"] = Root(work / "root.txt"); state["root"]["x"] = work / "n0"; state["hits"] = Hits(work / "hits.log"); state["hits"].clear()
    srv = subprocess.Popen([sys.executable, __file__, "--serve", str(port), str(work)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(200000):         # connect until the server listens (refused connections return at once)
        try: socket.create_connection(("127.0.0.1", port), timeout=1).close(); break
        except OSError: pass
    URL = f"http://127.0.0.1:{port}/"
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(); ctx.add_init_script(SKIP)
        pg = ctx.new_page()
        ver = lambda: pg.evaluate("window.MZLAB_VERSION")
        def first():
            pg.goto(URL); pg.wait_for_function("navigator.serviceWorker.controller !== null", timeout=30000)
            wait_cache(pg, app["n0"])
            V0.append(ver())
        step("build 0: the worker takes control and fills its cache", first)
        def reload_new():
            state["root"]["x"] = work / "n1"; state["hits"].clear()
            pg.reload(); pg.wait_for_load_state("load")
            assert ver() == "N1", ("normal reload after a deploy still shows", ver())
            body = pg.evaluate("fetch('static/version.js').then(r => r.text())")
            assert "N1" in body, body
        step("a normal reload after a deploy shows the new version at once", reload_new)
        def page_n_keeps_n():
            wait_cache(pg, app["n1"])
            keys = pg.evaluate("caches.keys()")
            assert not any(k.endswith(app["n0"]) for k in keys), keys
            assert any(k.endswith(app["n1"]) for k in keys), keys
        step("the old cache goes, the new one is filled", page_n_keeps_n)
        def cache_first():
            state["hits"].clear(); pg.reload(); pg.wait_for_load_state("load")
            assert ver() == "N1"
            h = static_hits()
            assert not h, ("static files asked to the server on a warm reload", list(h)[:8])
            assert state["hits"].get("/sw.js", 0) <= 1, state["hits"].get("/sw.js")
        step("a warm reload asks the server for no static file (cache-first)", cache_first)
        def no_mix():
            # the server is already N2, the page (N1) asks for an uncached-in-this-load file: it still gets N1 bytes from the cache of N1
            state["root"]["x"] = work / "n2"
            body = pg.evaluate("fetch('static/version.js').then(r => r.text())")
            assert "N1" in body, ("the page of build N1 got a file of build N2", body)
        step("the page of one build never gets a file of the next (cache of the worker that controls it)", no_mix)
        def update():
            pg.reload(); pg.wait_for_load_state("load")
            assert ver() == "N2", ver()
            wait_cache(pg, app["n2"])
            pg.reload(); pg.wait_for_load_state("load"); assert ver() == "N2"
            keys = pg.evaluate("caches.keys()")
            assert not any(k.endswith((app["n0"], app["n1"])) for k in keys), keys
        step("the next navigation takes build N2 and its worker replaces the cache", update)
        def reset_full():
            state["root"]["x"] = work / "n0"       # the server goes back to the first build: the newest one the user can reach is now N0
            r = pg.evaluate("QQRESET.wipe('all').then(x => x.sw)")
            state["hits"].clear()
            pg.goto(URL + "?riparti=1"); pg.wait_for_load_state("load")
            pg.wait_for_function("window.MZLAB_VERSION !== undefined")
            assert ver() == V0[0], ("Full reset kept a file of a newer or older build than the server's", ver())
            state["root"]["x"] = work / "n1"
            pg.evaluate("QQRESET.wipe('all')")
            pg.goto(URL + "?riparti=1"); pg.wait_for_load_state("load")
            assert ver() == "N1", ("Full reset did not bring the newest version", ver())
        step("«Full reset» brings the newest version although the HTTP cache holds the old files", reset_full)
        b.close()
    srv.terminate()
finally:
    shutil.rmtree(work, ignore_errors=True)
print("FAIL:", errs) if errs else print("OK e2e_sw_versione")
sys.exit(1 if errs else 0)
