"""mzFinder in the desktop shim, in Chromium, without Tauri: the page of the desktop (dist/, made by prepare.py: no Pyodide, no wasm-feat/), mzFinder on, against the real native
engine (`cargo run --example serve`, the mzlab:// protocol over HTTP). A synthetic high-resolution series chosen with «Choose mzML...» (the bytes go to the app once): classify, the Dig,
its details and the table of the page. The Dig equals the one of `cargo run --example finder` (timing aside) and no request names pyodide or wasm-feat. Run from the repository root:
  python3 tools/build_site.py --no-pyodide && python3 crates/mzlab-desktop/prepare.py && python3 crates/mzlab-desktop/tests/e2e_shim_finder.py"""
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
PARENT = {"name": "parent", "neutral": "C21H32N2O2", "adduct": "[M+H]+"}
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + (str(e) or repr(e))[:900]))

if not (DIST / "index.html").exists():
    print("  ", ("dist", "FAIL run tools/build_site.py --no-pyodide and crates/mzlab-desktop/prepare.py first")); sys.exit(1)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="shim_finder_"))
ser, nat = tmp / "ser", tmp / "nat"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
idx = sorted(ser.glob("*.mzidx"))
files = [f.with_suffix(".mzML") for f in idx]
nat.mkdir()
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "finder", "--", str(nat), *map(str, idx), "--parent", PARENT["neutral"],
                "--rules", str(ROOT / "TP_Mine" / "data" / "regole_tp.json")], cwd=ROOT, check=True, capture_output=True)
native = {"run": json.loads((nat / "run.json").read_text()), "detail": json.loads((nat / "detail.json").read_text())}
subprocess.run(["cargo", "build", "-q", "-p", "mzlab-desktop", "--example", "serve"], cwd=ROOT, check=True)
eng = subprocess.Popen([str(ROOT / "target" / "debug" / "examples" / "serve"), str(ENGINE_PORT)], stdout=subprocess.PIPE, text=True,
                       env={**os.environ, "MZLAB_LIBDIR": str(tmp / "Librerie"), "MZLAB_DATA": str(tmp / "app")})
for line in eng.stdout:
    if line.startswith("READY"): break

class Quiet(SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
page = ThreadingHTTPServer(("127.0.0.1", PAGE_PORT), partial(Quiet, directory=str(DIST)))
threading.Thread(target=page.serve_forever, daemon=True).start()

FAKE_TAURI = """window.__TAURI__ = { core: { invoke: async () => [] }, event: { listen: async () => () => {} }, window: { getCurrentWindow: () => ({ setTitle: () => {} }) } };"""
def native_route(route):                 # mzlab.localhost -> the engine over HTTP (what the window does with the custom protocol)
    u = route.request.url.replace("http://mzlab.localhost", f"http://127.0.0.1:{ENGINE_PORT}")
    try:
        data = route.request.post_data_buffer if route.request.method == "POST" else None
        with urllib.request.urlopen(urllib.request.Request(u, data=data, method=route.request.method)) as r: status, body, ctype = r.status, r.read(), r.headers["Content-Type"]
    except urllib.error.HTTPError as e: status, body, ctype = e.code, e.read(), e.headers["Content-Type"]
    route.fulfill(status=status, body=body, headers={"Content-Type": ctype, "Access-Control-Allow-Origin": "*"})

errs, reqs = [], []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        ctx = b.new_context(user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36", viewport={"width": 1280, "height": 900}, reduced_motion="reduce")
        ctx.add_init_script(FAKE_TAURI)
        ctx.add_init_script("try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        ctx.route(re.compile(r"http://mzlab\.localhost/.*"), native_route)
        ctx.on("request", lambda r: reqs.append(r.url))
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.TPCALL === 'function' && window.MZLAB_DESKTOP === true", timeout=120000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        got = {}
        def choose():
            pg.set_input_files("#tp-in", [str(f) for f in files])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= %d" % len(files), timeout=300000)
            m = pg.inner_text("#tp-msg"); assert "error" not in m.lower(), m
            info = pg.evaluate("([n]) => window.TPCALL('classify', n)", [[f.name for f in files]])
            assert [i["name"] for i in info] == [f.name for f in files] and all(i.get("hr_kind") == "hr" for i in info), info
            got["files"] = [{"name": i["name"], "type": i["type"], "time": i.get("time"), "kind": "hr"} for i in info]
        step("Choose mzML: the files go to the app, classify says high resolution", choose)
        def dig():
            run = pg.evaluate("([f, p]) => window.TPCALL('run', f, p, {}, '')", [got["files"], PARENT])
            a, n = dict(run), dict(native["run"])
            for k in ("timing", "settings"): a.pop(k, None); n.pop(k, None)
            assert a == n, [k for k in n if a.get(k) != n.get(k)]
            assert run["n_candidates"] >= 1 and set(run["timing"]) >= {"feature", "align", "proofs", "dig", "total"}, run["timing"]
        step("run: the Dig equals the native example's (timing aside)", dig)
        def details():
            for d in native["detail"]: assert pg.evaluate("([i]) => window.TPCALL('detail', i)", [d["id"]]) == d, d["id"]
        step("detail of the first candidates equals the native example's", details)
        def ui():
            pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            pg.locator("#hr-t tr.clk").first.click(); pg.wait_for_timeout(1500)
            m = pg.inner_text("#tp-msg"); assert "error" not in m.lower(), m
        step("the page: Dig, table and detail of a candidate", ui)
        def unsupported():
            r = pg.evaluate("([f, p]) => window.TPCALL('run', f, p, {}, 'a;+O').then(() => 'ran', e => String(e.message || e))", [got["files"], PARENT])
            assert "not in the app yet" in r, r
        step("a modified list of transformations says it is not in the app yet", unsupported)
        def clean():
            bad = [u for u in reqs if re.search(r"pyodide|wasm-feat|mzlab\.zip", u, re.I)]
            assert not bad, bad[:5]
        step("no request names pyodide, wasm-feat or mzlab.zip", clean)
        b.close()
finally:
    eng.terminate(); page.shutdown()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
