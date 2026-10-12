"""SCIEX .wiff + .wiff.scan in the page (Rust engine, flow without Pyodide, ?motore=rust&lr=1): a pair of synthetic files (tools/wiff_riferimento.py) is paired by
name and each sample becomes an mzML in the worker, as a .raw does; the samples open like any file (the scan sample with its TIC and spectra, the MRM sample with its
transitions) and Pyodide is never requested; a .wiff without its .wiff.scan gives a notice and no file. Needs the site (python tools/build_site.py) and the .wasm
(built here when cargo and wasm-bindgen exist; the CI builds it in site/static/wasm). No laboratory data is used."""
import shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

PORT = 8835
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "--config", 'target.wasm32-unknown-unknown.rustflags=["-C","target-feature=+simd128"]'], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

work = Path(tempfile.mkdtemp(prefix="e2e_wiff_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(work)], check=True)
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def pyodide(reqs): return [u for u in reqs if "/pyodide/" in u or u.endswith("mzlab.zip")]

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        ctx = b.new_context(viewport={"width": 1500, "height": 1800}); ctx.set_default_timeout(120000); reqs = []
        ctx.on("request", lambda r: reqs.append(r.url)); pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)

        def lonely():
            pg.set_input_files("#pick", [str(work / "lonely.wiff")])
            pg.wait_for_selector("#unopen:not([hidden])", timeout=60000)
            t = pg.inner_text("#unopen")
            assert "lonely.wiff" in t and ".wiff.scan" in t, t
            assert pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length") == 0
        step("a .wiff without its .wiff.scan gives a notice and no file", lonely)

        def pair():
            pg.set_input_files("#pick", [str(work / "two.wiff"), str(work / "two.wiff.scan")])
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            names = pg.evaluate("[...document.querySelectorAll('#flist input[data-k=use]')].map(i => i.closest('tr, li, div').innerText)")
            assert any("two_s1" in n for n in names) and any("two_s2" in n for n in names), names
            assert not pyodide(reqs), pyodide(reqs)[:3]
        step("two.wiff + two.wiff.scan: one mzML per sample, in the list, without Pyodide", pair)

        def open_them():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
            files = pg.evaluate(JSON, "api/state")["j"]
            assert len(files.get("files") or []) >= 2, files
            assert not pyodide(reqs), pyodide(reqs)[:3]
        step("the samples open (TIC drawn) without Pyodide", open_them)

        def mrm_and_scan():
            r = pg.evaluate(JSON, "api/mrm?k=1")
            assert r["status"] == 200 and r["j"]["files"][0]["transitions"], r
            t = pg.evaluate(JSON, "api/chrom?k=0&level=1")
            assert t["status"] == 200 and t["j"], t
            assert not pyodide(reqs), pyodide(reqs)[:3]
        step("the MRM sample has its transitions, the scan sample its TIC (Rust engine)", mrm_and_scan)

        def convert_back():
            n = pg.evaluate("CONV.size")
            assert n == 2, n
        step("the converted mzML files can be downloaded (CONV)", convert_back)
        assert not errs, errs[:3]
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(work, ignore_errors=True)
