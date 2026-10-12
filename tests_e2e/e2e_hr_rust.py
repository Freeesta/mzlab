"""High-resolution DDA file with the flow without Pyodide (?motore=rust&lr=1) on the built site: opening the file, the DDA table (api/dda) and the mean
spectrum of a window (api/spectrum, api/spectra, api/scan, api/scanavg: survey, product ions, one scan read from the mzML) come from the Rust engine and no request of those routes is left to
Python (window.MZLAB_LR_MISS; api/contaminants, a static list, still is). Needs the site and the .wasm (python tools/build_site.py)."""
import subprocess, sys, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT
from lib_hr import synth

PORT = 8837
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "SKIP site/ or site/static/wasm not built")); sys.exit(0)
D = synth("hrrust")
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
MISS = 'window.MZLAB_LR_MISS || []'
def missed(pg): return [m for m in pg.evaluate(MISS) if not m.startswith(("GET api/contaminants", "POST api/notebook"))]   # api/contaminants (a static list) is still Python's, not part of this package
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
        c.on("request", lambda r: reqs.append(r.url)); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop", timeout=120000)
        def load():
            pg.set_input_files("#pick", [str(D / "HR_DDA-Exploris-t30.mzML")])
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000); pg.wait_for_timeout(3000)
            assert not missed(pg), missed(pg)
        step("a high-resolution DDA file opens without a request left to Python, with the DDA trio on screen", load)
        def dda():
            r = pg.evaluate(JSON, "api/dda?k=0")
            assert r["status"] == 200 and r["eng"] == "rust" and len(r["j"]["sid"]) > 50 and r["j"]["nce"] is True, (r["status"], r["eng"], list(r["j"])[:4])
            assert len(r["j"]["ms1"]["sid"]) > 50
        step("api/dda comes from the Rust engine", dda)
        def spectra():
            for q in ("rt0=14.2&rt1=14.4&level=1", "rt0=0&rt1=1000&level=2"):
                r = pg.evaluate(JSON, f"api/spectrum?k=0&{q}")
                assert r["status"] == 200 and r["eng"] == "rust" and len(r["j"]["mz"]) > 100 and r["j"]["scans"] > 1, (q, r["status"], r["eng"])
            assert not missed(pg), missed(pg)
        step("api/spectrum of a window (survey and product ions) comes from the Rust engine", spectra)
        def single():
            sid = pg.evaluate(JSON, "api/scanlist?k=0&level=1")["j"]
            rt = sid["rt"][len(sid["rt"]) // 2]
            r = pg.evaluate(JSON, f"api/spectrum?k=0&rt0={rt - 1e-4}&rt1={rt + 1e-4}&level=1")
            assert r["status"] == 200 and r["j"]["scans"] == 1 and r["eng"] == "rust" and len(r["j"]["mz"]) > 20, (r["status"], r["eng"])
            assert not missed(pg), missed(pg)
        step("a window of one scan is read from the mzML by Rust, still without a request left to Python", single)
        def scans():
            for q in ("api/spectra?k=0&level=1&i0=0&i1=2", "api/spectra?k=1&level=2&i0=0&i1=3"):
                r = pg.evaluate(JSON, q)
                assert r["status"] == 200 and r["eng"] == "rust" and len(r["j"]["scans"]) >= 3 and r["j"]["scans"][0]["mode"] == "centroid" and len(r["j"]["scans"][0]["mz"]) > 0, (q, r["status"], len(r["j"]["scans"]), r["j"]["scans"][0]["mz"][:3], r["eng"])
            d = pg.evaluate(JSON, "api/dda?k=0")["j"]
            r = pg.evaluate(JSON, f"api/scan?k=0&sid={d['sid'][5]}")
            assert r["status"] == 200 and r["eng"] == "rust" and r["j"]["level"] == 2 and len(r["j"]["mz"]) > 5 and r["j"]["parent"] is not None, (r["status"], r["eng"])
            r = pg.evaluate(JSON, "api/scanavg?k=0&sids=" + ",".join(str(x) for x in d["sid"][3:9]))
            assert r["status"] == 200 and r["eng"] == "rust" and r["j"]["n"] == 6 and len(r["j"]["mz"]) > 5, (r["status"], r["eng"])
            assert not missed(pg), missed(pg)
        step("api/spectra, api/scan and api/scanavg come from the Rust engine (scans read from the mzML by offset), without a request left to Python", scans)
        b.close()
finally:
    srv.terminate()
if errs: print("  ", ("pageerror", "; ".join(errs)[:400]))
