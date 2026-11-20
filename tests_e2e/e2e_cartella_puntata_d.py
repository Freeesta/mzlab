"""«Open folder» (the pointed folder, cartella-puntata.js) reads folder datasets: a folder holding a synthetic GC/MS .D (data.ms on top, AcqData with Contents.xml only), a .D without
data.ms and an .mzML lists the .D folders as rows (not entered), a click loads each .D as one file through the same code as a dropped folder, and the incomplete one gets the notice that
lists the files received. showDirectoryPicker is replaced by a folder of the browser's private file system (a real FileSystemDirectoryHandle). The same folder through the fallback of
browsers without the API (Safari, Firefox: the icon opens the one-time folder input, webkitdirectory) loads the same files. Synthetic folders of tools/agilent_sintetico.py (no laboratory data).
Needs the site and the .wasm, as e2e_agilent_gc_datams.py."""
# covers: mzlab/web/cartella-puntata.js, mzlab/web/cartella-carica.js, mzlab/web/agilent-d.js
import base64, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, free_port
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico
sys.path.insert(0, str(ROOT / "tests_e2e"))
import synth

PORT = free_port()
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

D = Path(tempfile.mkdtemp(prefix="cartp_d_e2e_"))
top = D / "lab"
synth.write_full(top / "a.mzML", 5, seed=1)
agilent_sintetico.write_old(top / "serie", "gcsynth.D", 60, gc=True)
agilent_sintetico.write_old(top / "serie/cut", "incompleta.D", 60, gc=True)
(top / "serie/cut/incompleta.D/data.ms").unlink()
FILES = {str(f.relative_to(top)): base64.b64encode(f.read_bytes()).decode() for f in sorted(top.rglob("*")) if f.is_file()}
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: srv.poll() is None or sys.exit("server did not start"); threading.Event().wait(0.1)
INIT = """(() => { if (localStorage.getItem('nopicker')) { delete window.showDirectoryPicker; return; }
  window.showDirectoryPicker = async () => (await navigator.storage.getDirectory()).getDirectoryHandle('lab', { create: true }); })()"""
PUT = """async ([path, b64]) => { let d = await (await navigator.storage.getDirectory()).getDirectoryHandle('lab', { create: true }); const parts = path.split('/'); const name = parts.pop();
  for (const p of parts) d = await d.getDirectoryHandle(p, { create: true });
  const w = await (await d.getFileHandle(name, { create: true })).createWritable(); await w.write(Uint8Array.from(atob(b64), c => c.charCodeAt(0))); await w.close(); }"""
def rows(pg): return pg.evaluate("Array.from(document.querySelectorAll('#cartbody .cprow .cpn')).map(e=>e.textContent)")
def names(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0].replace(/\.mzml$/i,''))")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        def page(nopicker=False):
            c = b.new_context(viewport={"width": 1500, "height": 2000}); c.set_default_timeout(120000); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');" + ("localStorage.setItem('nopicker','1');" if nopicker else "") + "}catch(_){}")
            pg.add_init_script(INIT)
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#cartpick", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            return pg
        pg = page()
        for rel, b64 in FILES.items(): pg.evaluate(PUT, [rel, b64])
        def listed():
            pg.click("#cartpick"); pg.wait_for_selector("#cartbody .cprow")
            assert rows(pg) == ["a.mzML", "serie/cut/incompleta.D", "serie/gcsynth.D"], rows(pg)
        step("the .D folders are rows of the list, not entered (no data.ms or Contents.xml row)", listed)
        def load_d():
            pg.click('.cprow:has-text("gcsynth.D")')
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            assert names(pg) == ["gcsynth"], names(pg)
            pg.wait_for_function("Array.from(document.querySelectorAll('.cprow.done .cpn')).some(e => e.textContent.endsWith('gcsynth.D'))")
            assert "has no AcqData" not in pg.inner_text("#up"), pg.inner_text("#up")
        step("a click on a .D row loads it as one file and the row turns «loaded»", load_d)
        def notice():
            pg.click('.cprow:has-text("incompleta.D")')
            pg.wait_for_function("/has no AcqData/.test(document.querySelector('#up').innerText)", timeout=60000)
            t = pg.inner_text("#up"); assert "AcqData/Contents.xml (" in t and "GC.ini (" in t, t
        step("a .D without data.ms gets the notice that lists the files received", notice)
        def mzml():
            pg.click('.cprow:has-text("a.mzML")')
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
        step("the .mzML of the same folder loads beside it", mzml)
        pg2 = page(nopicker=True)
        def fallback():
            with pg2.expect_file_chooser() as fc: pg2.click("#cartpick")
            assert fc.value.is_multiple(), "no showDirectoryPicker: the icon opens the one-time folder input (webkitdirectory)"
            pg2.set_input_files("#pickdir", str(top))
            pg2.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            assert sorted(names(pg2)) == ["a", "gcsynth"], names(pg2)
        step("without the File System Access API (Safari, Firefox) the one-time folder input loads the same .D and .mzML", fallback)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
for e in errs: print("  ", e)
sys.exit(1 if errs else 0)
