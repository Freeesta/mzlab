"""An Agilent MassHunter .D folder loaded with «Load a folder» (cartella-carica.js + agilent-d.js), on the built site with the Rust engine and WITHOUT Pyodide
(?motore=rust&lr=1): the folder becomes one mzML (named like the folder), listed and indexed like any file, with its TIC; a .D without MSPeak.bin gives a notice;
two .D folders give two files. Synthetic folders of tools/agilent_sintetico.py (no laboratory data).
Needs the site and the .wasm: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm)."""
import os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico
sys.path.insert(0, str(ROOT / "tests_e2e"))
import synth

PORT = 8837
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm"], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="agilent_e2e_"))
top = D / "serie"
agilent_sintetico.write(top, "uno.D", 120)
agilent_sintetico.write(top / "sub", "due.D", 90)
broken = agilent_sintetico.write(top, "rotto.D", 30)
(broken / "AcqData/MSPeak.bin").unlink()
synth.write_full(top / "mix.mzML", 5, seed=1)
agilent_sintetico.write_old(top)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
def pyodide(reqs): return [u for u in reqs if "/pyodide/" in u or u.endswith("mzlab.zip")]
def names(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
        c.on("request", lambda r: reqs.append(r.url)); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        def folder():
            pg.set_input_files("#pickdir", str(top))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 4", timeout=120000)
            n = sorted(names(pg)); assert n == ["due.mzML", "mix.mzML", "uno.mzML", "vecchia.mzML"], n
        step("a folder with two .D runs, an older .D with only data.ms and an mzML lists four files", folder)
        def notice():
            t = pg.inner_text("#up")
            assert "rotto.D" in t, t
        step("the .D without MSPeak.bin gets a notice", notice)
        def tidy():
            pg.wait_for_function("document.querySelector('#cartinfo').hidden || !document.querySelector('#cartinfo').textContent", timeout=60000)       # «Folder …: n files to load» goes away when the load is done
            assert ".D" in pg.inner_text("h3[data-i18n=\"load.step1.titleNative\"]") and ".D" in pg.inner_text("#drop")
            w = pg.evaluate("({s: document.querySelector('#start').getBoundingClientRect().width, g: document.querySelector('#flist tr.grp td').getBoundingClientRect().width, t: document.querySelector('#flist table').getBoundingClientRect().width})")
            assert w["s"] > 1000 and w["g"] >= w["t"] - 2, w                                                                             # wide panel; the group bar spans the whole table
        step("loaded: no leftover «files to load» line, .D in the title and the drop text, wide panel, full-width group bar", tidy)
        def tic():
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
            r = pg.evaluate("""async u => { const r = await fetch(u); return { status: r.status, j: await r.json() }; }""", "api/chrom?k=0&level=1")
            j = r["j"]; assert r["status"] == 200, r
            y = j["y"]; t = j["rt"]
            assert len(y) in (90, 120) and max(y) > 5 * min(y), (len(y), min(y), max(y))
            assert 3.0 <= min(t) and max(t) <= 3.41, (min(t), max(t))
        step("the converted run answers the TIC route", tic)
        def noPyodide():
            assert not pyodide(reqs), pyodide(reqs)[:3]
        step("Pyodide is never requested", noPyodide)
        c.close(); b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
for e in errs: print("  ", e)
sys.exit(1 if errs else 0)
