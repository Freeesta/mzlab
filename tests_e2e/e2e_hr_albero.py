"""MSn tree (api/msntree) of a synthetic direct-infusion MSn file with the flow without Pyodide (?motore=rust&lr=1) on the built site: the spectra are read from
the mzML by offset in the Rust engine, Pyodide is not downloaded, nothing is left in window.MZLAB_LR_MISS, and the JSON equals the classic engine's.
Needs the site and the .wasm (python tools/build_site.py)."""
import subprocess, sys, tempfile, time
from pathlib import Path
import numpy as np
from playwright.sync_api import sync_playwright
from lib import ROOT

sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
PORT = 8839
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:700]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "SKIP site/ or site/static/wasm not built")); sys.exit(0)
import dati_sintetici as ds  # noqa: E402
FILE = Path(tempfile.mkdtemp(prefix="e2e_albero_")) / "MSN_infusion.mzML"
ds.msn_infusion(FILE, np.random.default_rng(5))
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
# api/contaminants (the built-in lists) is still Python's and would start Pyodide when the file opens: answered empty here
NO_LISTS = """(() => { const nf = window.fetch.bind(window); let cur = nf;
  Object.defineProperty(window, "fetch", { configurable: true, get: () => cur, set: f => { cur = (u, i) => String((u && u.url) || u).includes("api/contaminants") ? Promise.resolve(new Response('{"lists":[]}', { headers: { "Content-Type": "application/json" } })) : f(u, i); } }); })();"""
def json_close(a, b, path=""):
    if isinstance(a, dict) and isinstance(b, dict):
        assert a.keys() == b.keys(), f"{path}: keys {sorted(a.keys() ^ b.keys())}"
        for k in a: json_close(a[k], b[k], f"{path}.{k}")
    elif isinstance(a, list) and isinstance(b, list):
        assert len(a) == len(b), f"{path}: length {len(a)} against {len(b)}"
        for i, (x, y) in enumerate(zip(a, b)): json_close(x, y, f"{path}[{i}]")
    elif isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        assert abs(a - b) <= 1e-6 * max(1.0, abs(a), abs(b)), f"{path}: {a} against {b}"
    else: assert a == b, f"{path}: {a!r} against {b!r}"
def open_file(pg):
    pg.wait_for_selector("#drop", timeout=120000)
    pg.set_input_files("#pick", [str(FILE)])
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000); pg.wait_for_timeout(2000)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        def context(init=None):
            c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
            c.on("request", lambda r: reqs.append(r.url)); pg = c.new_page()
            if init: pg.add_init_script(init)
            return pg, reqs
        pg, reqs = context(NO_LISTS); pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); open_file(pg)
        pg2, _ = context(); pg2.goto(f"http://127.0.0.1:{PORT}/?motore=python"); open_file(pg2)
        def tree():
            k = pg.evaluate("E.files.findIndex(f => f.kind === 'ms2')"); assert k >= 0, k
            for q in (f"api/msntree?k={k}", f"api/msntree?k={k}&formula=C16H17N2O4&ppm=3"):
                ra, rb = pg.evaluate(JSON, q), pg2.evaluate(JSON, q)
                assert ra["status"] == rb["status"] == 200 and ra["eng"] == "rust", (q, ra["status"], rb["status"], ra["eng"])
                assert ra["j"]["n_nodes"] >= 3, ra["j"]["n_nodes"]
                json_close(ra["j"], rb["j"], q)
            assert not [u for u in reqs if "/pyodide/" in u or u.endswith("mzlab.zip")], "Pyodide requested"
            assert not [m for m in pg.evaluate("window.MZLAB_LR_MISS || []") if "msntree" in m]
        step("the MSn tree of a synthetic infusion comes from the Rust engine, equal to Python's, without Pyodide", tree)
        b.close()
finally:
    srv.terminate()
