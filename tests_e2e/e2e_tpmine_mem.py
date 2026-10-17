"""TP Mine HR: memory of the real worker (Pyodide) with the 13 real HR files of the series (80 MB each, mounted as Blobs) + the MSn file.
The WebAssembly memory only grows, so its size at the end is the peak. Target: < 1 GB. Needs QQQ_DATI (private data).
Run from the repo root: python3 tests_e2e/e2e_tpmine_mem.py   (minutes)"""
import glob, json, os, re, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)
SRC = Path(os.environ.get("TPMINE_SRC", ROOT / "TP_Mine"))
DATI = Path(os.environ["QQQ_DATI"]) if os.environ.get("QQQ_DATI") else None
GB = 1 << 30
if not DATI or not (DATI / "HRMS").is_dir():
    print("SKIP: QQQ_DATI with HRMS/ not available"); sys.exit(0)
truth = next((json.loads(f.read_text(encoding="utf-8")) for f in sorted((DATI / "scratchpad").glob("*_truth.json")) if "tps" in f.read_text(encoding="utf-8")), None)
msn = next((json.loads(f.read_text(encoding="utf-8")) for f in sorted((DATI / "scratchpad").glob("*_truth.json")) if "nodes" in f.read_text(encoding="utf-8")), None)
series = sorted(glob.glob(str(DATI / "HRMS" / "*" / "*TiO2*.mzML")))
if truth is None or msn is None or len(series) < 13:
    print("SKIP: truth files or the 13 series files not available"); sys.exit(0)
from lib import free_port; PORT = free_port()
tmp = Path(tempfile.mkdtemp())
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; ok = False


try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1400}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        files = sorted(series, key=lambda f: float(re.search(r"t(\d+)min", f).group(1)) if "min" in f else -1) + [str(DATI / msn["file"])]
        t0 = time.time()
        pg.set_input_files("#tp-in", files)
        pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 15", timeout=1200000)
        print(f"files loaded in {time.time() - t0:.0f} s; wasm memory after loading: {pg.evaluate('window.TPMINE_MEM()')['wasm'] / GB:.2f} GB")
        pg.fill("#tp-smi", truth["parent"]["smiles"]); pg.wait_for_function("document.querySelector('#tp-prev').textContent.includes('317.1642')", timeout=60000)
        t0 = time.time()
        pg.click("#tp-go")
        while not pg.evaluate("!!document.querySelector('#hr-t tr.clk')"):
            if time.time() - t0 > 3600: raise SystemExit("timeout: the run takes more than an hour")
            pg.wait_for_timeout(3000)
            m = pg.inner_text("#tp-msg")
            if m.strip() and "errore" in m.lower(): raise SystemExit(m)
        took = time.time() - t0
        wasm = pg.evaluate("window.TPMINE_MEM()")["wasm"]
        n_rows = pg.evaluate("document.querySelectorAll('#hr-t tr.clk').length")
        print(f"run in the worker: {took:.0f} s, {n_rows} rows; WebAssembly memory (peak) {wasm / GB:.2f} GB")
        ok = wasm < GB and n_rows > 100
        print("OK" if ok else f"FAIL: peak of the WebAssembly memory {wasm / GB:.2f} GB (target < 1 GB) or too few rows")
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
print("page errors:", errs)
sys.exit(0 if ok and not errs else 1)
