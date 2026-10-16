"""Measure the Dig of Tabui over N synthetic low-resolution files in K conditions (2 files each: t0 and t10), the first draw of the view «Compare conditions» and the peak memory of the browser.
The files are the ones of tests/data/gruppi/ copied under new names (the run id is changed so that every file has its own bytes). Usage (repo root, a built site with static/wasm-feat):
  MZLAB_PIPE_SITE=<site> python3 tools/misura_gruppi.py [20 40]   -> one JSON line per N: {n, conditions, dig_s, view_ms, rss_peak_mb}"""
import json, os, re, subprocess, sys, tempfile, threading, time, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
ROOT = Path(__file__).resolve().parent.parent; HERE = ROOT / "tests_e2e"
sys.path.insert(0, str(HERE)); import lib  # noqa: E402

DATA = ROOT / "tests" / "data" / "gruppi"; SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"
SRC = [f"{c}_{t}_r1.mzML" for c in ("Air", "Ar", "He", "N2", "O2", "Reference") for t in ("t0", "t10")]

def make(n, out):
    out.mkdir(parents=True, exist_ok=True); paths = []
    for k in range(n // 2):
        for j, t in enumerate(("t0", "t10")):
            src = (DATA / SRC[(k % 6) * 2 + j]).read_text(); name = f"Cond{k + 1:02d}_{t}_r1.mzML"
            (out / name).write_text(re.sub(r'(<run id=")[^"]*"', rf'\g<1>{name[:-5]}"', src, count=1)); paths.append(str(out / name))
    return paths

def rss_mb():
    tot = 0
    for p in Path("/proc").glob("[0-9]*"):
        try:
            if b"pw-browsers" in (p / "cmdline").read_bytes(): tot += int((p / "statm").read_text().split()[1]) * 4096
        except OSError: pass
    return tot / 2**20

def run(n, site, port):
    paths = make(n, Path(tempfile.mkdtemp(prefix="mgr_"))); peak = [0.0]; stop = threading.Event(); gate = threading.Event()
    def watch():
        while not stop.is_set():
            if gate.is_set(): peak[0] = max(peak[0], rss_mb())
            time.sleep(0.15)
    threading.Thread(target=watch, daemon=True).start()
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 1100}, reduced_motion="reduce"); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.admin','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{port}/"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.set_input_files("#pick", paths); pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=480000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=480000); pg.wait_for_function(f"E.files.length >= {n} && E.pending.length === 0", timeout=900000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go"); pg.wait_for_function("document.querySelectorAll('#tp-files tr').length > 1", timeout=60000)
        pg.click("#tp-cond > summary"); pg.wait_for_selector("#tp-cfiles table .tpg-cond", timeout=120000); pg.click("#tp-cfiles button:has-text('Confirm conditions')")
        pg.fill("#tp-mol", SMILES); time.sleep(1); gate.set(); t0 = time.time(); pg.click("#tp-go"); pg.wait_for_selector("#tp-t", timeout=1500000); dig = time.time() - t0; gate.clear()
        ms = pg.evaluate("""async () => { const b = document.querySelector('.vtabs .vt[data-v="cond"]'), t = performance.now(); b.click();
          while (!document.querySelector('#gv-mx tbody tr')) await new Promise(r => requestAnimationFrame(r)); return performance.now() - t; }""")
        conds = pg.evaluate("document.querySelectorAll('#gv-mx thead button[data-sort]').length")
        b.close()
    stop.set()
    return {"n": n, "conditions": n // 2, "dig_s": round(dig, 1), "view_ms": round(ms), "rss_peak_mb": round(peak[0]), "matrix_cols": conds}

if __name__ == "__main__":
    site = os.environ["MZLAB_PIPE_SITE"]; port = lib.free_port()
    srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(port), site], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(100):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1).close(); break
        except Exception: time.sleep(0.2)
    try:
        for n in [int(a) for a in sys.argv[1:]] or [20, 40]: print(json.dumps(run(n, site, port)), flush=True)
    finally: srv.terminate()
