"""A Thermo .raw given again in the desktop page (the shim and the native engine, as in e2e_shim.py): the same file picked twice keeps its one sample row and
its one index (nativo.js cached -> natsame of rpc_nat.rs), in the same run and in a new run of the app on the same folders. Synthetic .raw (crates/mzlab-lettori
examples/sintetici, invented numbers). Run from the repository root:
  python3 tools/build_site.py && python3 crates/mzlab-desktop/prepare.py && python3 crates/mzlab-desktop/tests/e2e_shim_raw.py"""
import json, os, re, shutil, subprocess, sys, tempfile, threading, urllib.request
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tests_e2e"))
import lib  # noqa: F401,E402  (its wait_for_function polls: the page CSP forbids eval)

DIST = ROOT / "crates" / "mzlab-desktop" / "dist"
PAGE_PORT, ENGINE_PORT = 8851, 8852
fails = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: fails.append(name); print("  ", (name, "FAIL " + (str(e) or repr(e))[:600]))

if not (DIST / "index.html").exists():
    print("  ", ("dist", "FAIL run tools/build_site.py and crates/mzlab-desktop/prepare.py first")); sys.exit(1)
SRC = Path(tempfile.mkdtemp(prefix="mzlab-raw-"))
subprocess.run(["cargo", "run", "-q", "--release", "-p", "mzlab-lettori", "--example", "sintetici", "--", str(SRC)], cwd=ROOT, check=True)
RAW = SRC / "thermo" / "synt.raw"
subprocess.run(["cargo", "build", "-q", "-p", "mzlab-desktop", "--example", "serve"], cwd=ROOT, check=True)
DATA = Path(tempfile.mkdtemp(prefix="mzlab-data-"))                    # the folder of the app: files.json, the indexes

def engine():
    e = subprocess.Popen([str(ROOT / "target" / "debug" / "examples" / "serve"), str(ENGINE_PORT), str(RAW)], stdout=subprocess.PIPE, text=True,
                         env={**os.environ, "MZLAB_LIBDIR": str(DATA / "Librerie"), "MZLAB_APP_DIR": str(DATA)})
    pick = None
    for line in e.stdout:
        if line.startswith("FILE "): _, i, name, size = line.split(); pick = {"id": int(i), "name": name, "size": int(size)}
        if line.startswith("READY"): break
    return e, pick

class Quiet(SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
page = ThreadingHTTPServer(("127.0.0.1", PAGE_PORT), partial(Quiet, directory=str(DIST)))
threading.Thread(target=page.serve_forever, daemon=True).start()
FAKE_TAURI = """try { if (!sessionStorage.getItem("mzlab.run")) sessionStorage.setItem("mzlab.run", "1"); } catch (_) {}
try { localStorage.setItem("mzlab.upd.last", String(Date.now())); } catch (_) {}
window.__TAURI__ = { core: { invoke: async (cmd, a) => cmd === "pick_files" ? (a.dam ? [] : %s) : cmd === "update_check" ? JSON.stringify({ status: "uptodate", version: "x" }) : [] },
  event: { listen: async () => () => {} }, window: { getCurrentWindow: () => ({ setTitle: () => {} }) } };"""
def native(route):                    # mzlab.localhost -> the engine over HTTP (what the window does with the custom protocol)
    u = route.request.url.replace("http://mzlab.localhost", f"http://127.0.0.1:{ENGINE_PORT}")
    try:
        data = route.request.post_data_buffer if route.request.method == "POST" else None
        with urllib.request.urlopen(urllib.request.Request(u, data=data, method=route.request.method)) as r: status, body, ctype = r.status, r.read(), r.headers["Content-Type"]
    except urllib.error.HTTPError as e: status, body, ctype = e.code, e.read(), e.headers["Content-Type"]
    route.fulfill(status=status, body=body, headers={"Content-Type": ctype, "Access-Control-Allow-Origin": "*"})

# upload() counted: a loading has ended when the count grows (the shim hands the picked files to the global upload)
COUNT = "(() => { if (window.__ups === undefined) { const u = window.upload; window.__ups = 0; window.upload = async (...a) => { try { return await u(...a); } finally { window.__ups++; } }; } return window.__ups; })()"
IDLE = "UPBUSY === 0 && OPENNOW.busy() === 0"
def state(pg):
    return {"rows": pg.evaluate("ST.files.map(f => f.name).sort()"), "idx": sorted(p.name for p in DATA.rglob("*.mzidx"))}
def give(pg):
    n = pg.evaluate(COUNT)
    pg.click("#drop")
    pg.wait_for_function(f"window.__ups > {n} && {IDLE}", timeout=180000)
def run(pg_of):
    eng, pick = engine()
    try:
        with sync_playwright() as p:
            b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
            ctx = b.new_context(); ctx.add_init_script(FAKE_TAURI % json.dumps([pick])); ctx.route(re.compile(r"http://mzlab\.localhost/.*"), native)
            pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)[:200]))
            pg.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html")
            pg.wait_for_function("window.MZLAB_DESKTOP === true && typeof ST !== 'undefined' && window.qqReady === true", timeout=120000)
            out = pg_of(pg)
            assert not errs, errs
            b.close()
            return out
    finally:
        eng.terminate(); eng.wait()
first = {}
def same_run():
    def go(pg):
        give(pg)
        pg.wait_for_function(f"ST.files.length === 1 && OPENNOW.isOpen(ST.files[0].name) && {IDLE}", timeout=180000)
        first.update(state(pg))
        assert len(first["idx"]) >= 1, first
        give(pg)
        assert state(pg) == first, ("same run", first, state(pg))
    run(go)
step("the same .raw picked twice keeps one row and one index", same_run)
def new_run():
    def go(pg):
        pg.wait_for_function(f"ST.files.length === 1 && {IDLE}", timeout=180000)
        give(pg)
        assert state(pg) == first, ("new run", first, state(pg))
    run(go)
step("in a new run of the app, the same .raw picked again keeps one row and one index", new_run)
page.shutdown(); shutil.rmtree(SRC, ignore_errors=True); shutil.rmtree(DATA, ignore_errors=True)
sys.exit(1 if fails else 0)
