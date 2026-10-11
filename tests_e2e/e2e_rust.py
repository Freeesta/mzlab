"""Rust engine (?motore=rust) on the built site: TIC, XIC and single scans answered by mzlab-wasm give the same values as Python (Pyodide),
and a worker that dies is restarted by the supervisor (a second death within a minute sends everything back to Python).
Needs the site: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm)."""
import shutil, subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, mz

PORT = 8832
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:400]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "--config", 'target.wasm32-unknown-unknown.rustflags=["-C","target-feature=+simd128"]'], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
GET = """async u => { const r = await fetch(u); return { eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def close(a, b, tol, rel=0.0):
    assert len(a) == len(b), (len(a), len(b))
    bad = [(i, x, y) for i, (x, y) in enumerate(zip(a, b)) if abs(x - y) > max(tol, rel * abs(y))]
    assert not bad, f"{len(bad)} values differ, first {bad[:3]}"
def json_close(a, b, tol=1e-9):
    if isinstance(a, dict) and isinstance(b, dict): return a.keys() == b.keys() and all(json_close(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list): return len(a) == len(b) and all(json_close(x, y) for x, y in zip(a, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool): return abs(a - b) <= tol * max(1.0, abs(a), abs(b))
    return a == b
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None); ctx = b.new_context(viewport={"width": 1500, "height": 1800}); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.on("response", lambda r: errs.append(("http%d" % r.status, r.url)) if r.status >= 400 else None)
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust")
        def boot():
            pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_function("window.MZLAB_RUST !== undefined", timeout=30000)
        step("engine loads and the Rust bridge is there", boot)
        def load():
            pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15"), mz("B_MRM-t0")])
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 4", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("window.MZLAB_RUST.stats.rust > 0", timeout=60000)
            pg.wait_for_function("E.files.length >= 4 && E.pending.length === 0", timeout=120000)          # the list is opened file by file: wait for the last one
        step("open the files: the TIC panels are drawn by Rust", load)
        def one_reader():
            # a file has one reader: the TIC panels came from the .mzidx, no mzML was read by the Rust engine
            opened = pg.evaluate("window.MZLAB_RUST.stats.mzmlOf || []")
            assert all("MRM" in n for n in opened), opened                  # only the MRM file (no scans, no index of the peaks) is read as an mzML
        step("the TIC came from the index: no mzML read (but the MRM file)", one_reader)
        def tic_xic():
            n = pg.evaluate("E.files.length"); assert n >= 4, n
            seen = set()
            pg.evaluate("window.MZLAB_RUST.enabled = true")
            for k in range(n):
                for url in (f"api/chrom?k={k}&kind=tic&level=1", f"api/xic?k={k}&mz=364&tol=0.35&level=1"):
                    pg.evaluate("window.MZLAB_RUST.enabled = true"); r = pg.evaluate(GET, url)
                    pg.evaluate("window.MZLAB_RUST.enabled = false"); py = pg.evaluate(GET, url)
                    assert py["eng"] is None
                    seen.add(r["eng"])
                    ra, pa = (r["j"], py["j"]) if "traces" not in r["j"] else (r["j"]["traces"][0], py["j"]["traces"][0])
                    close(ra["rt"], pa["rt"], 2e-4); close(ra["y"], pa["y"], 0.11, 1e-6)           # Rust keeps intensities in float32
            pg.evaluate("window.MZLAB_RUST.enabled = true")
            assert "rust" in seen, seen                       # at least the Full Scan files went through Rust
        step("TIC and XIC: same values as Python on every file", tic_xic)
        def scans():
            url = "api/spectra?k=0&i0=0&i1=6&level=1&bin=0.1"
            pg.evaluate("window.MZLAB_RUST.enabled = true"); r = pg.evaluate(GET, url); pg.evaluate("window.MZLAB_RUST.enabled = false"); py = pg.evaluate(GET, url); pg.evaluate("window.MZLAB_RUST.enabled = true")
            assert r["eng"] == "rust" and py["eng"] is None, (r["eng"], py["eng"])
            assert r["j"]["n"] == py["j"]["n"] and len(r["j"]["scans"]) == len(py["j"]["scans"])
            for a, c in zip(r["j"]["scans"], py["j"]["scans"]):
                assert (a["i"], a["sid"], a["mode"]) == (c["i"], c["sid"], c["mode"]), (a["i"], c["i"])
                close([a["rt"]], [c["rt"]], 2e-4); close(a["mz"], c["mz"], 1.1e-3); close(a["y"], c["y"], 0.11, 1e-6)
        step("single scans: same peaks as Python", scans)
        def index_routes():
            # the routes the .mzidx answers (no mzML is read): same JSON as Python, on every file that has scans (the MRM file is Python's)
            n = pg.evaluate("E.files.length"); rust_n = 0
            for k in range(n):
                f = pg.evaluate(GET, f"api/filters?k={k}")
                urls = [f"api/chrom?k={k}&kind=bpc&level=1", f"api/chrom?k={k}&kind=tic&level=1&mz0=150&mz1=400", f"api/chrom?k={k}&kind=bpc&level=1&mz0=200",
                        f"api/filters?k={k}", f"api/scanlist?k={k}&level=1", f"api/scanlist?k={k}&level=2", f"api/nearest_scan?k={k}&rt=4.2&level=1", f"api/nearest_scan?k={k}&rt=9&level=2", f"api/scaninfo?k={k}&sid=0", f"api/scaninfo?k={k}&sid=2"]
                for url in urls:
                    pg.evaluate("window.MZLAB_RUST.enabled = true"); r = pg.evaluate(GET, url)
                    pg.evaluate("window.MZLAB_RUST.enabled = false"); py = pg.evaluate(GET, url)
                    pg.evaluate("window.MZLAB_RUST.enabled = true")
                    if r["eng"] == "rust":
                        rust_n += 1
                        assert json_close(r["j"], py["j"]), (url, str(r["j"])[:200], str(py["j"])[:200])
                    else:
                        assert py["eng"] is None
            assert rust_n >= 8, rust_n
        step("index routes (chrom with m/z window and bpc, filters, scanlist, nearest_scan, scaninfo): same JSON as Python", index_routes)
        def other_routes():
            r = pg.evaluate(GET, "api/spectra?k=0&i0=0&i1=2&level=1&filt=x"); assert r["eng"] is None       # a filter is left to Python
            r = pg.evaluate(GET, "api/chrom?k=0&kind=pda&level=1"); assert r["eng"] is None
        step("what Rust cannot answer exactly goes to Python", other_routes)
        def thermo_raw():
            import os
            raws = [p for d in [os.environ.get("MZLAB_DATI"), str(ROOT.parent / "mzlab-dati")] if d for p in __import__("glob").glob(os.path.join(d, "HRMS", "*", "*.raw"))]
            if not raws: print("   (skipped: no .raw in the data repository)"); return
            assert pg.is_visible("#dropraw"), "the .raw note is not shown"
            n0 = len(pg.query_selector_all("#flist input[data-k=use]"))
            pg.set_input_files("#pick", raws[0])
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length > {n0}", timeout=240000)
            assert pg.evaluate("ST.files.some(f => /\\.mzML$/.test(f.name) && f.name.startsWith(%r))" % os.path.splitext(os.path.basename(raws[0]))[0])
        step("a Thermo .raw is read by the engine and listed as an mzML", thermo_raw)
        def die_once():
            pg.evaluate("window.MZLAB_RUST.crash()")
            pg.wait_for_function("window.MZLAB_RUST.stats.restarts === 1", timeout=30000)
            assert pg.inner_text("#rust-note").strip(), "no message"
            r = pg.evaluate(GET, "api/chrom?k=0&kind=tic&level=1"); assert r["eng"] == "rust", r["eng"]      # files reopened by the supervisor
        step("worker dies: restarted, files reopened, message shown", die_once)
        def die_twice():
            pg.evaluate("window.MZLAB_RUST.crash()")
            pg.wait_for_function("window.MZLAB_RUST.disabled === true", timeout=30000)
            r = pg.evaluate(GET, "api/chrom?k=0&kind=tic&level=1"); assert r["eng"] is None and len(r["j"]["rt"]) > 3
        step("second death within a minute: back to Python", die_twice)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
