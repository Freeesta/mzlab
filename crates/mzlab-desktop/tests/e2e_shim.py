"""Desktop shim in Chromium, without Tauri: the page of the desktop (dist/, made by prepare.py), the shim and the native worker against the
real native engine (`cargo run --example serve`, the mzlab:// protocol over HTTP), with a fake `window.__TAURI__` for the file dialog.
Same checks as e2e_rust: TIC and XIC from the native engine equal Python's (Pyodide). Run from the repository root:
  python3 tools/build_site.py && python3 crates/mzlab-desktop/prepare.py && python3 crates/mzlab-desktop/tests/e2e_shim.py"""
import json, os, re, subprocess, sys, tempfile, threading, time, urllib.request
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tests_e2e"))
import lib  # noqa: F401,E402  (its wait_for_function polls: the page CSP forbids eval)
DIST = ROOT / "crates" / "mzlab-desktop" / "dist"
FILE = ROOT / "mzlab" / "web" / "esempi" / "FullScan_t10.mzML"
PAGE_PORT, ENGINE_PORT = 8841, 8842
fails = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: fails.append(name); print("  ", (name, "FAIL " + str(e)[:400]))

if not (DIST / "index.html").exists() or not FILE.exists():
    print("  ", ("dist", "FAIL run tools/build_site.py and crates/mzlab-desktop/prepare.py first")); sys.exit(1)

subprocess.run(["cargo", "build", "-q", "-p", "mzlab-desktop", "--example", "serve"], cwd=ROOT, check=True)
LIBDIR = Path(tempfile.mkdtemp(prefix="mzlab-libs-")) / "Librerie"      # the native folder of the libraries (does not exist yet: the app creates it)
eng = subprocess.Popen([str(ROOT / "target" / "debug" / "examples" / "serve"), str(ENGINE_PORT), str(FILE)], stdout=subprocess.PIPE, text=True, env={**os.environ, "MZLAB_LIBDIR": str(LIBDIR)})
picked = []
for line in eng.stdout:
    if line.startswith("FILE "):
        _, i, name, size = line.split(); picked.append({"id": int(i), "name": name, "size": int(size)})
    if line.startswith("READY"): break

class Quiet(SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
page = ThreadingHTTPServer(("127.0.0.1", PAGE_PORT), partial(Quiet, directory=str(DIST)))
threading.Thread(target=page.serve_forever, daemon=True).start()

FAKE_TAURI = """window.__UPD = { check: {status: "uptodate", version: "x"}, calls: [], progress: [], restarted: false, listeners: [], delay: 0 };
try { localStorage.setItem("qqq.upd.last", String(Date.now())); } catch (_) {}       // the daily silent check stays out of the way: the steps drive it
window.__TAURI__ = { core: { invoke: async (cmd, a) => {
    if (cmd === "pick_files") return a.dam ? [] : %s;
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
  window: { getCurrentWindow: () => ({ setTitle: () => {} }) } };""" % json.dumps(picked)
def native(route):                    # mzlab.localhost -> the engine over HTTP (what the window does with the custom protocol)
    u = route.request.url.replace("http://mzlab.localhost", f"http://127.0.0.1:{ENGINE_PORT}")
    try:
        data = route.request.post_data_buffer if route.request.method == "POST" else None
        with urllib.request.urlopen(urllib.request.Request(u, data=data, method=route.request.method)) as r: status, body, ctype = r.status, r.read(), r.headers["Content-Type"]
    except urllib.error.HTTPError as e: status, body, ctype = e.code, e.read(), e.headers["Content-Type"]
    route.fulfill(status=status, body=body, headers={"Content-Type": ctype, "Access-Control-Allow-Origin": "*"})

GET = """async u => { const r = await fetch(u); return { eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def close(a, b, tol, rel=0.0):
    assert len(a) == len(b), (len(a), len(b))
    bad = [(i, x, y) for i, (x, y) in enumerate(zip(a, b)) if abs(x - y) > max(tol, rel * abs(y))]
    assert not bad, f"{len(bad)} values differ, first {bad[:3]}"

try:
    with sync_playwright() as p:
        import os; b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        ctx = b.new_context(user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36")
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
            pg.click("#drop")                                    # the shim opens the (fake) system dialog and hands the file to upload()
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("window.MZLAB_RUST.stats.rust > 0", timeout=60000)
        step("a file picked in the dialog is opened and its TIC is drawn by the native engine", load)
        def tic_xic():
            for url in ("api/chrom?k=0&kind=tic&level=1", "api/xic?k=0&mz=364&tol=0.35&level=1", "api/xic?k=0&mz=300&tol=0.5&level=1&px=200"):
                pg.evaluate("window.MZLAB_RUST.enabled = true"); r = pg.evaluate(GET, url)
                pg.evaluate("window.MZLAB_RUST.enabled = false"); py = pg.evaluate(GET, url)
                assert r["eng"] == "rust" and py["eng"] is None, (url, r["eng"], py["eng"])
                if "px=" in url: continue                        # a reduced trace has no Python twin to compare with
                ra, pa = (r["j"], py["j"]) if "traces" not in r["j"] else (r["j"]["traces"][0], py["j"]["traces"][0])
                close(ra["rt"], pa["rt"], 2e-4); close(ra["y"], pa["y"], 0.11, 1e-6)
            pg.evaluate("window.MZLAB_RUST.enabled = true")
        step("TIC and XIC from the native engine: same values as Python", tic_xic)
        def scans():
            url = "api/spectra?k=0&i0=0&i1=6&level=1&bin=0.1"
            pg.evaluate("window.MZLAB_RUST.enabled = true"); r = pg.evaluate(GET, url)
            pg.evaluate("window.MZLAB_RUST.enabled = false"); py = pg.evaluate(GET, url); pg.evaluate("window.MZLAB_RUST.enabled = true")
            assert r["eng"] == "rust" and py["eng"] is None
            assert r["j"]["n"] == py["j"]["n"] and len(r["j"]["scans"]) == len(py["j"]["scans"])
            for a, c in zip(r["j"]["scans"], py["j"]["scans"]):
                close(a["mz"], c["mz"], 1.1e-3); close(a["y"], c["y"], 0.11, 1e-6)
        step("single scans from the native engine: same values as Python", scans)
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
