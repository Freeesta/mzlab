"""Desktop shim in Chromium, without Tauri: the page of the desktop (dist/, made by prepare.py), the shim and the native worker against the real native engine
(`cargo run --example serve`, the mzlab:// protocol over HTTP), with a fake `window.__TAURI__` for the file dialog. The answers of every route the page asks of
its engine equal the Python reference's (tests_e2e/pyref.py); nothing is left to Python (window.MZLAB_LR_MISS stays empty, the app carries none). Run from
the repository root:
  python3 tools/build_site.py && python3 crates/mzlab-desktop/prepare.py && python3 crates/mzlab-desktop/tests/e2e_shim.py"""
import json, os, re, subprocess, sys, tempfile, threading, time, urllib.request
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tests_e2e"))
import lib  # noqa: F401,E402  (its wait_for_function polls: the page CSP forbids eval)
from pyref import PyRef  # noqa: E402
DIST = ROOT / "crates" / "mzlab-desktop" / "dist"
EX = ROOT / "mzlab" / "web" / "esempi"
LR_FILES = [EX / "FullScan_t10.mzML", EX / "FullScan_t30.mzML", EX / "MS2_t15.mzML", EX / "MRM_std_2.4ppm.mzML"]
from lib_hr import synth  # noqa: E402  (synthetic high-resolution DDA files)
HR_FILES = [synth("shimhr") / "HR_DDA-Exploris-t30.mzML", synth("shimhr") / "HR_DDA-Fusion-t30.mzML"]
PAGE_PORT, ENGINE_PORT = 8841, 8842
fails = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e:
        fails.append(name); print("  ", (name, "FAIL " + (str(e) or repr(e))[:600]))
        if os.environ.get("SHIM_DEBUG"): import traceback; traceback.print_exc()

if not (DIST / "index.html").exists() or not all(f.exists() for f in LR_FILES + HR_FILES):
    print("  ", ("dist", "FAIL run tools/build_site.py and crates/mzlab-desktop/prepare.py first")); sys.exit(1)

subprocess.run(["cargo", "build", "-q", "-p", "mzlab-desktop", "--example", "serve"], cwd=ROOT, check=True)
LIBDIR = Path(tempfile.mkdtemp(prefix="mzlab-libs-")) / "Librerie"      # the native folder of the libraries (does not exist yet: the app creates it)
DATA = Path(tempfile.mkdtemp(prefix="mzlab-data-"))                    # the folder of the app: files.json, kv.json, the indexes
eng = subprocess.Popen([str(ROOT / "target" / "debug" / "examples" / "serve"), str(ENGINE_PORT), *map(str, LR_FILES + HR_FILES)], stdout=subprocess.PIPE, text=True, env={**os.environ, "MZLAB_LIBDIR": str(LIBDIR), "MZLAB_DATA": str(DATA)})
picked = {}
for line in eng.stdout:
    if line.startswith("FILE "):
        _, i, name, size = line.split(); picked[name] = {"id": int(i), "name": name, "size": int(size)}
    if line.startswith("READY"): break
LR_PICK = [picked[f.name] for f in LR_FILES]; HR_PICK = [picked[f.name] for f in HR_FILES]

class Quiet(SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
page = ThreadingHTTPServer(("127.0.0.1", PAGE_PORT), partial(Quiet, directory=str(DIST)))
threading.Thread(target=page.serve_forever, daemon=True).start()

FAKE_TAURI = """window.__UPD = { check: {status: "uptodate", version: "x"}, calls: [], progress: [], restarted: false, listeners: [], delay: 0 };
try { localStorage.setItem("qqq.upd.last", String(Date.now())); } catch (_) {}       // the daily silent check stays out of the way: the steps drive it
window.__TAURI__ = { core: { invoke: async (cmd, a) => {
    if (cmd === "pick_files") return a.dam ? [] : (window.__PICK || %s);
    if (cmd === "update_check") { window.__UPD.calls.push(cmd); if (window.__UPD.delay) await new Promise(r => setTimeout(r, window.__UPD.delay)); return JSON.stringify(window.__UPD.check); }
    if (cmd === "update_apply") {
      window.__UPD.calls.push(cmd);
      for (let i = 1; i <= 3; i++) for (const f of window.__UPD.listeners) f({ payload: { done: i, total: 3, bytes: i * 1e6, total_bytes: 3e6 } });
      await new Promise(r => setTimeout(r, 50));
      return JSON.stringify({ status: "ready", files: 3 });
    }
    if (cmd === "update_restart") { window.__UPD.restarted = true; return null; }
    return [];
  } },
  event: { listen: async (name, f) => { if (name === "update-progress") window.__UPD.listeners.push(f); return () => {}; } },
  window: { getCurrentWindow: () => ({ setTitle: () => {} }) } };""" % json.dumps(LR_PICK)
def native(route):                    # mzlab.localhost -> the engine over HTTP (what the window does with the custom protocol)
    u = route.request.url.replace("http://mzlab.localhost", f"http://127.0.0.1:{ENGINE_PORT}")
    try:
        data = route.request.post_data_buffer if route.request.method == "POST" else None
        with urllib.request.urlopen(urllib.request.Request(u, data=data, method=route.request.method)) as r: status, body, ctype = r.status, r.read(), r.headers["Content-Type"]
    except urllib.error.HTTPError as e: status, body, ctype = e.code, e.read(), e.headers["Content-Type"]
    route.fulfill(status=status, body=body, headers={"Content-Type": ctype, "Access-Control-Allow-Origin": "*"})

JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def json_close(a, b, tol=1e-9, path=""):
    if isinstance(a, dict) and isinstance(b, dict):
        assert a.keys() == b.keys(), f"{path}: keys {sorted(a.keys() ^ b.keys())}"
        for k in a: json_close(a[k], b[k], tol, f"{path}.{k}")
    elif isinstance(a, list) and isinstance(b, list):
        assert len(a) == len(b), f"{path}: length {len(a)} against {len(b)}"
        for i, (x, y) in enumerate(zip(a, b)): json_close(x, y, tol, f"{path}[{i}]")
    elif isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        assert abs(a - b) <= tol * max(1.0, abs(a), abs(b)), f"{path}: {a} against {b}"
    else: assert a == b, f"{path}: {a!r} against {b!r}"
ctx_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36"

try:
    with sync_playwright() as p:
        import os; b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        ctx = b.new_context(user_agent=ctx_ua)
        ctx.add_init_script(FAKE_TAURI)
        ctx.route(re.compile(r"http://mzlab\.localhost/.*"), native)
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: fails.append("pageerror " + str(e)[:200]))
        pg.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html")
        def boot():
            pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_function("window.MZLAB_RUST !== undefined && window.MZLAB_DESKTOP === true", timeout=30000)
        step("the page loads with the desktop shim and the native bridge", boot)
        def load():
            pg.click("#drop")                                    # the shim opens the (fake) system dialog and hands the files to upload()
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(LR_FILES)}", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function(f"E.files.length >= {len(LR_FILES)} && E.pending.length === 0", timeout=120000)
            pg.wait_for_function("window.MZLAB_RUST.stats.rust > 0", timeout=60000)
            assert not pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_MISS")
        step("files picked in the dialog are opened and the TIC is drawn by the native engine", load)
        def native_listing():                                    # a dataset the readers do not know is refused through the whole way: bridge, shim, rpc, readers
            f = LR_PICK[0]
            r = pg.evaluate("""async f => { const file = new File([], "a.bin", { type: "application/x-mzlab-file;id=" + f.id }); Object.defineProperty(file, "size", { value: f.size });
              try { await window.MZLAB_RUST.nativeSamples([{ name: "a.bin", blob: file }]); return "listed"; } catch (e) { return (e.json && e.json.error_key) || String(e); } }""", f)
            assert r != "listed" and r.startswith("err."), r
        step("natsamples reaches the readers of the native engine and refuses a file they do not know", native_listing)
        ref = {}
        def state_same():
            sess = pg.evaluate(JSON, "api/session")["j"]
            ref["names"] = [it["file"] for it in sess["session"]]
            pg.wait_for_function("fetch('api/notebook').then(r => r.json()).then(n => (n.session || []).length >= %d)" % len(LR_FILES), timeout=30000)      # saved a moment after the last change
            nb = pg.evaluate("fetch('api/notebook').then(r => r.json())")["session"]
            ref["py"] = PyRef([str(EX / n.split("#")[0]) for n in dict.fromkeys(ref["names"])], nb)
            py = {"state": ref["py"].get("api/state")["j"], "session": ref["py"].get("api/session")["j"]}
            st = pg.evaluate(JSON, "api/state")["j"]
            # the engine reads the infusion MSn of a file (`msn`, mzlab-core/src/infusione.rs); the Python reference does not
            nomsn = lambda x: {k: nomsn(v) for k, v in x.items() if k != "msn"} if isinstance(x, dict) else [nomsn(v) for v in x] if isinstance(x, list) else x
            json_close(nomsn(st["files"]), py["state"]["files"], path="state.files")
            json_close(nomsn({k: v for k, v in sess.items() if k not in ("workdir", "commit")}), {k: v for k, v in py["session"].items() if k not in ("workdir", "commit")}, path="session")
        step("api/state and api/session: the JSON of the Python reference", state_same)
        def routes_same():
            names = ref["names"]; k = {n.split("#")[0]: i for i, n in reversed(list(enumerate(names)))}
            full, full2, ms2, mrm = k["FullScan_t10.mzML"], k["FullScan_t30.mzML"], k["MS2_t15.mzML"], k["MRM_std_2.4ppm.mzML"]
            urls = [f"api/chrom?k={full}&kind=tic&level=1", f"api/chrom?k={full}&kind=bpc&level=1&mz0=150&mz1=400", f"api/xic?k={full},{full2}&mz=364&tol=0.35&level=1",
                    f"api/filters?k={full}", f"api/scanlist?k={full}&level=1", f"api/nearest_scan?k={full}&rt=9&level=1", f"api/scaninfo?k={full}&sid=3",
                    f"api/spectrum?k={full}&rt0=14&rt1=14.7&level=1&bin=0.1", f"api/spectra?k={full}&i0=100&i1=104&level=1&bin=0.1", f"api/scan?k={full}&sid=300",
                    f"api/fileinfo?k={full}", f"api/ms2tr?k={ms2}", f"api/scan_axis?k={full}&level=1", f"api/mrm?k={mrm}", f"api/contaminants",
                    "api/formula?f=C6H12O6", "api/formula?f=Xx2", "api/composition?mz=233.0243&elements=C%3A0-20%2CH%3A0-40%2CN%3A0-5%2CO%3A0-8&tol=5&unit=ppm&max=10",
                    f"api/map?k={full}", f"api/map?k={full2}&bg=1&agg=mean", f"api/map?k={full}&ref={full2}&norm=max", f"api/mapsnap?k={full}&rt=14.25&mz=364.4",
                    f"api/mappunti?k={full}&ref={full2}&pts=14.3,364.3;14.32,194.2",
                    f"api/origin?k={full}&mz=194.2&parent=364.37", f"api/chrom?k={full2}&ref={full}", f"api/xic?k={full2}&ref={full}&mz=364.4&tol=0.35", f"api/mrm?k={mrm}&ref={mrm}",
                    f"api/qaqc?k={full},{full2}&roles=sample,blank&mz=364.4&rt=14.3&ratio=1", "api/method_warnings", f"api/gradient?k={full}", f"api/delay?k={full}", f"api/chrom?k={full}&kind=pda",
                    f"api/chrom?k={mrm}", f"api/chrom?k={mrm}&kind=bpc", f"api/chrom?k={mrm}&kind=srm", f"api/xic?k={mrm}&mz=200&tol=0.35", f"api/filters?k={mrm}", f"api/scanlist?k={mrm}"]
            bad = []
            for u in urls:
                a = pg.evaluate(JSON, u); b2 = ref["py"].get(u)
                if a["status"] != b2["status"]: bad.append((u, a["status"], b2["status"], str(a["j"])[:120])); continue
                if a["status"] == 200:
                    for r in (a["j"], b2["j"]):
                        if isinstance(r, dict): r.pop("timing", None)
                    try: json_close(a["j"], b2["j"], 1e-6, u)
                    except AssertionError as e: bad.append(str(e)[:200])
                if a["eng"] != "rust": bad.append((u, "engine", a["eng"]))
            assert not bad, bad[:4]
            POST = """async ([u, b]) => { const r = await fetch(u, { method: "POST", body: JSON.stringify(b) }); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
            pts = [{"file": f"f{i}", "time": t, "rt": 5.0, "area": 100.0 * (i + 1), "height": 9.0 * i} for i, t in enumerate([30, 0, 60, 15])]
            for u, body in [("api/stat/profile", {"points": pts, "metric": "area"}), ("api/gradient_parse", {"fmt": "csv", "text": "0,5\n5,95\n10,95\n11,5"})]:
                a = pg.evaluate(POST, [u, body]); b2 = ref["py"].post(u, body)
                assert a["status"] == b2["status"] == 200 and a["eng"] == "rust", (u, a["status"], b2["status"])
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_MISS")
        step("every route of the low-resolution flow: the JSON of the Python reference, nothing left to Python", routes_same)
        def persists():
            assert (DATA / "files.json").is_file() and (DATA / "kv.json").is_file() and list((DATA / "mzidx").glob("*.mzidx")), list(DATA.rglob("*"))
            pg.wait_for_function("fetch('api/notebook').then(r => r.json()).then(n => (n.session || []).length >= %d)" % len(LR_FILES), timeout=30000)
            pg.reload(); pg.wait_for_function("[...document.querySelectorAll('.pnl canvas')].some(c => c.checkVisibility())", timeout=120000)      # the first canvas may be a hidden one
            pg.wait_for_function("E.files.length >= 1 && window.MZLAB_LR !== undefined", timeout=120000)
            assert sorted(f["name"] for f in pg.evaluate(JSON, "api/state")["j"]["files"]) == sorted(f.name for f in LR_FILES)
        step("the files and the notebook are still there after a reload (the app's folder), as references to the files on the disk", persists)
        def dragged():                                           # a File that only exists in the page (not picked in the dialog): its bytes go to the app once
            pg.set_input_files("#pick", [str(EX / "FullScan_t5.mzML")])
            pg.wait_for_function("fetch('api/state').then(r => r.json()).then(s => s.files.some(f => f.name === 'FullScan_t5.mzML'))", timeout=60000)
            assert (DATA / "files" / "FullScan_t5.mzML").is_file()
            pg.evaluate("fetch('api/remove', { method: 'POST', body: JSON.stringify({ name: 'FullScan_t5.mzML' }) })")
            pg.wait_for_function("fetch('api/state').then(r => r.json()).then(s => !s.files.some(f => f.name === 'FullScan_t5.mzML'))", timeout=30000)
            assert not (DATA / "files" / "FullScan_t5.mzML").exists()
        step("a File of the page (no dialog) is sent to the app as bytes, and removing it removes the copy", dragged)
        def demo():                                              # «Prova con i file di esempio»: the page fetches the demo mzML from its own bundle and sends them to the app as bytes
            c3 = b.new_context(user_agent=ctx_ua); c3.add_init_script(FAKE_TAURI); c3.route(re.compile(r"http://mzlab\.localhost/.*"), native)
            pg3 = c3.new_page(); pg3.on("pageerror", lambda e: fails.append("pageerror " + str(e)[:200]))
            pg3.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html"); pg3.wait_for_function("window.MZLAB_RUST !== undefined && window.MZLAB_DESKTOP === true", timeout=120000)
            pg3.evaluate("fetch('api/new', { method: 'POST', body: JSON.stringify({ fresh: true }) })"); pg3.reload(); pg3.wait_for_selector("#demobtn", timeout=120000)
            pg3.click("#demobtn"); pg3.click("[data-demo=mrm]")
            pg3.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 4", timeout=120000)
            assert not pg3.evaluate("window.MZLAB_LR_MISS || []")
            c3.close()
            c4 = b.new_context(user_agent=ctx_ua); c4.add_init_script(FAKE_TAURI)         # a webview that drops the body of a POST (WKWebView with a Blob): the message must say the file did not arrive
            def no_body(route):
                if "lr/upload" not in route.request.url: return native(route)
                route.fulfill(status=400, body=json.dumps({"error": "x", "error_key": "err.file.unknown", "params": {"detail": "empty upload"}}), headers={"Content-Type": "application/json", "Access-Control-Allow-Origin": "*"})
            c4.route(re.compile(r"http://mzlab\.localhost/.*"), no_body)
            pg4 = c4.new_page(); pg4.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html"); pg4.wait_for_function("window.MZLAB_DESKTOP === true", timeout=120000)
            pg4.evaluate("fetch('api/new', { method: 'POST', body: JSON.stringify({ fresh: true }) })"); pg4.reload(); pg4.wait_for_selector("#demobtn", timeout=120000)
            pg4.click("#demobtn"); pg4.click("[data-demo=mrm]")
            pg4.wait_for_timeout(8000); pg4.wait_for_function("document.body.innerText.includes('non ha ricevuto')", timeout=60000)
            assert "Thermo" not in pg4.inner_text("body")
            c4.close()
        step("the demo files load in the app (fetched from the bundle, sent as bytes)", demo)
        def hr():
            c2 = b.new_context(user_agent=ctx_ua); c2.add_init_script(FAKE_TAURI + "\nwindow.__PICK = %s;" % json.dumps(HR_PICK)); c2.route(re.compile(r"http://mzlab\.localhost/.*"), native)
            pg2 = c2.new_page(); pg2.on("pageerror", lambda e: fails.append("pageerror " + str(e)[:200]))
            pg2.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html?riparti=1"); pg2.wait_for_selector("#drop", timeout=120000)
            pg2.click("#drop"); pg2.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(HR_FILES)}", timeout=120000)
            pg2.click("#opbtn"); pg2.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg2.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000); pg2.wait_for_timeout(1500)
            sess = pg2.evaluate(JSON, "api/session")["j"]
            py = PyRef([str(f) for f in HR_FILES])
            items = sess["session"]
            ms1 = [k for k, it in enumerate(items) if "MS1" in it["label"] or it.get("kind") == "full"]
            ka, kc = ms1[0], ms1[1]
            sid = py.get(f"api/dda?k={ka}")["j"]["sid"]
            urls = [f"api/dda?k={ka}", f"api/spectrum?k={ka}&rt0=0&rt1=1000&level=1", f"api/spectrum?k={ka}&rt0=0&rt1=1000&level=2", f"api/spectra?k={ka}&level=1&i0=0&i1=2", f"api/spectra?k={ka}&level=2&i0=0&i1=3",
                    f"api/scan?k={ka}&sid={sid[5]}", f"api/scanavg?k={ka}&sids=" + ",".join(str(x) for x in sid[3:9]), f"api/map?k={ka}", f"api/map?k={ka}&ref={kc}&norm=max", f"api/mapsnap?k={ka}&rt=14.3&mz=364.07",
                    f"api/mappunti3d?k={ka}&rt0=14&rt1=14.6&mz0=363&mz1=367&max=300", f"api/origin?k={ka}&mz=305.0702&parent=364.0737", f"api/subxic?k={ka}&f=C14H14F4N3O2S&ppm=5&min=150",
                    f"api/fileinfo?k={ka}", f"api/scanlist?k={ka}&level=1", f"api/spectrum?k={ka}&rt0=14.2&rt1=14.6&level=1&bgk=0&bgrt0=10.1&bgrt1=10.9"]
            bad = []
            for u in urls:
                a = pg2.evaluate(JSON, u); b2 = py.get(u)
                if a["status"] != b2["status"]: bad.append((u, a["status"], b2["status"], str(a["j"])[:100])); continue
                if a["status"] == 200:
                    for r in (a["j"], b2["j"]):
                        if isinstance(r, dict): r.pop("timing", None)
                    try: json_close(a["j"], b2["j"], 1e-6, u)
                    except AssertionError as e: bad.append(str(e)[:200])
            assert not bad, bad[:4]
            r = pg2.evaluate("""async u => { const r = await fetch(u); const b = new Uint8Array(await r.arrayBuffer()); return { status: r.status, size: b.length }; }""", f"api/scanbin?k={ka}&level=1&i0=0&i1=3")
            assert r["status"] == 200 and r["size"] > 100, r
            assert not pg2.evaluate("window.MZLAB_LR_MISS || []"), pg2.evaluate("window.MZLAB_LR_MISS")
            c2.close()
        step("high-resolution files: dda, spectra, scans, maps, origin, sub-formulas, the binary block: the Python reference's JSON, nothing left to Python", hr)
        def upd_set(check):
            pg.evaluate("c => { window.__UPD.check = c; window.__UPD.calls.length = 0; window.__UPD.progress.length = 0; }", check)
        def gear_check():
            pg.evaluate("document.querySelector('#updbox') && document.querySelector('#updbox').remove()")
            pg.click("#np-set"); pg.wait_for_selector("#uip-upd"); pg.click("#uip-upd")
            pg.wait_for_function("document.querySelector('#updbox') && !/…/.test(document.querySelector('#updbox').textContent)", timeout=10000)
            return pg.inner_text("#updbox")
        def t(key, **kw): return pg.evaluate("([k, kw]) => I18N.t(k, kw)", [key, kw])
        def upd_available():
            upd_set({"status": "available", "version": "b", "files": 3, "bytes": 3000000})
            txt = gear_check()
            assert t("update.available", files=3, size="3.0") in txt or "3" in txt, txt
            pg.click("#updbox button[data-a=go]")
            pg.wait_for_selector("#updbox button[data-a=restart]", timeout=10000)
            assert pg.evaluate("window.__UPD.calls") == ["update_check", "update_apply"]
            assert t("update.ready") in pg.inner_text("#updbox")
            pg.click("#updbox button[data-a=restart]")
            pg.wait_for_function("window.__UPD.restarted === true")
        step("settings: 'check for updates' offers the download, shows the progress, then 'restart now'", upd_available)
        def upd_uptodate():
            upd_set({"status": "uptodate", "version": "b"})
            assert t("update.uptodate") in gear_check()
        step("settings: up to date", upd_uptodate)
        def upd_offline():
            upd_set({"status": "error", "error_key": "update.error.offline", "detail": "no route"})
            assert t("update.error.offline") in gear_check()
        step("settings: no internet gives a clear message, not an error", upd_offline)
        def upd_newapp():
            upd_set({"status": "newapp", "min": "9.9.9", "url": "https://github.com/Freeesta/mzlab/actions/workflows/desktop.yml"})
            gear_check()
            assert pg.evaluate("document.querySelector('#updbox a').href").startswith("https://github.com/Freeesta/mzlab/")
            assert "9.9.9" in pg.inner_text("#updbox")
        step("settings: a web part that needs a newer app points to the installers", upd_newapp)
        def upd_silent():
            pg.evaluate("document.querySelector('#updbox') && document.querySelector('#updbox').remove()")
            upd_set({"status": "uptodate", "version": "b"})
            pg.evaluate("DESKUPD.check(true)"); pg.wait_for_function("window.__UPD.calls.length === 1")
            assert pg.evaluate("document.querySelector('#updbox')") is None
            upd_set({"status": "available", "version": "c", "files": 1, "bytes": 1000})
            pg.evaluate("DESKUPD.check(true)"); pg.wait_for_selector("#updbox")
            assert t("update.notice") in pg.inner_text("#updbox")
        def upd_click_during_silent():          # the manual click while the silent check is in flight is not lost: it shows the same answer as a dialog
            pg.evaluate("document.querySelector('#updbox') && document.querySelector('#updbox').remove()")
            upd_set({"status": "available", "version": "d", "files": 2, "bytes": 2000000})
            pg.evaluate("() => { window.__UPD.delay = 1500; DESKUPD.check(true); }"); pg.wait_for_function("window.__UPD.calls.length === 1")
            pg.click("#np-set"); pg.wait_for_selector("#uip-upd"); pg.click("#uip-upd")
            pg.wait_for_selector("#updbox button[data-a=go]", timeout=10000)
            assert pg.evaluate("window.__UPD.calls.length") == 1, pg.evaluate("window.__UPD.calls"); assert t("update.notice") not in pg.inner_text("#updbox"), pg.inner_text("#updbox")
            pg.evaluate("window.__UPD.delay = 0")
        step("settings: a click during the silent check takes it over", upd_click_during_silent)
        step("silent check: nothing when up to date, a small notice when an update exists", upd_silent)
        # ---- the native folder of the libraries (crates/mzlab-desktop/src/librerie.rs behind lib/*, read by libreria-worker.js with the same code as the browsers)
        def rec(name, prec): return f"Name: {name}\nPrecursorMZ: {prec}\nIon_mode: Positive\nPrecursor_type: [M+H]+\nNum Peaks: 4\n60.5\t100\n100.1\t400\n150.2\t900\n200.3\t300\n"
        Q = [[60.5, 100], [100.1, 400], [150.2, 900], [200.3, 300]]
        def states(): return pg.evaluate("LIB.call('list').then(r => ({ mode: r.folder.mode, dir: r.folder.name, v: r.voci.map(v => [v.nome, v.tipo, v.stato]) }))")
        def native_empty():
            pg.wait_for_function("window.LIB !== undefined"); s_ = states()
            assert s_["mode"] == "native" and s_["v"] == [] and s_["dir"].endswith("Librerie"), s_
        step("the native folder is empty (it does not even exist yet) and the page knows it is the native one", native_empty)
        def native_index():
            LIBDIR.mkdir(parents=True); (LIBDIR / "a.msp").write_text(rec("Match", 300.1) + "\n" + rec("Lontano", 400.1), encoding="utf-8")
            assert states()["v"] == [["a.msp", "sorgente", "da_indicizzare"]]
            r = pg.evaluate("LIB.call('index', { name: 'a.msp' })"); assert r["info"]["n"] == 2, r
            assert states()["v"] == [["a.msp", "sorgente", "pronta"]]
            assert (LIBDIR / ".mzlab" / "a.msp.mzlib").stat().st_size > 100 and (LIBDIR / ".mzlab" / "a.msp.idx.json").is_file()
            assert not list(LIBDIR.rglob("*.part")), "no partial file stays"
            r = pg.evaluate("LIB.call('search', { peaks: %s, prec: 300.1, pol: 1, tol: { v: 10, unit: 'ppm' }, frag: 0.01 }).then(r => r.results.map(x => x.name))" % json.dumps(Q)); assert r == ["Match"], r
        step("a source of the folder is indexed into `.mzlab/` on the disk and searched (read memory-mapped by the native side)", native_index)
        def native_changed():
            (LIBDIR / "a.msp").write_text(rec("Match", 300.1) + "\n" + rec("Nuovo", 600.1), encoding="utf-8")
            assert states()["v"] == [["a.msp", "sorgente", "da_indicizzare"]]
        step("a changed source is «to index» again", native_changed)
        def native_damaged():
            pg.evaluate("LIB.call('index', { name: 'a.msp' })"); assert states()["v"][0][2] == "pronta"
            idx = LIBDIR / ".mzlab" / "a.msp.mzlib"; idx.write_bytes(idx.read_bytes()[:40]); assert states()["v"][0][2] == "danneggiata"
            pg.evaluate("LIB.call('index', { name: 'a.msp' })"); pg.evaluate("LIB.call('remove', { id: 'f:a.msp' })")
            assert (LIBDIR / "a.msp").is_file() and states()["v"] == [["a.msp", "sorgente", "da_indicizzare"]]
        step("a truncated index is «damaged», indexing again mends it, removing it leaves the source", native_damaged)
        b.close()
finally:
    eng.kill(); page.shutdown()
print("  ", ("result", "FAIL " + ", ".join(fails) if fails else "ok"))
sys.exit(1 if fails else 0)
