"""mzFinder, low resolution: files opened in the Data tab with the Rust engine live in OPFS (not IndexedDB); «Reload from Data» must find them.
Synthetic Full Scan series (TP_Mine/tests/lr_synth.py). Run from the repo root: python3 tests_e2e/e2e_tpmine_lr_reload.py"""
import os, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401
from lib import free_port; PORT = free_port()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:400]))
tmp = Path(tempfile.mkdtemp())
sys.path[:0] = [str(ROOT / "TP_Mine" / "tests"), str(ROOT / "TP_Mine" / "py")]
import lr_synth
files = lr_synth.write_series(tmp / "lr", times=(0, 15, 60), blank=False)
paths = [str(f["path"]) for f in files]
cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")]
if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
subprocess.run(cmd, check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")          # the engine of the page is Rust: the .wasm goes in the copy of the site
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(tmp / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1500, "height": 1100}).new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_timeout(3200)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=30000)
        pg.set_input_files("#pick", paths)
        pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=120000)
        def reload_lr():
            pg.wait_for_timeout(3000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.click("#tp-page")
            pg.wait_for_function(f"document.querySelectorAll('#tp-files tr').length >= {len(paths) + 1}", timeout=240000)
            t = pg.evaluate("document.getElementById('tp-files').textContent")
            assert "Full Scan" in t or "full" in t.lower(), t
            assert not pg.evaluate("(document.getElementById('tp-msg')||{}).className||''").count("err")
        step("Reload from Data lists the LR Full Scan files of the Data tab", reload_lr)
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(("OK  " if r == "ok" else "FAIL"), n, "" if r == "ok" else r)
if errs: print("page errors:", errs[:3])
sys.exit(0 if all(r == "ok" for _, r in steps) and not errs else 1)
