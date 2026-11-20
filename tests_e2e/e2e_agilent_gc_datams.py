"""A GC/MS Agilent .D folder shaped like a copy without its converted index (data.ms at the top, AcqData holding only Contents.xml, side files beside it), loaded by «browse»
(the folder input) and by a dropped folder (simulated entries in the real drop handler): each loads as one file, no «not a whole .D» notice. A folder with no data.ms gets the notice
that lists what was received, with sizes. «Open folder» (the pointed folder) does not read .D folders yet, so it is not covered. Synthetic folders of tools/agilent_sintetico.py (no laboratory data).
Needs the site and the .wasm, as e2e_agilent.py."""
# covers: mzlab/web/agilent-d.js, tools/agilent_sintetico.py, mzlab/web/rust-nativo, mzlab/web/nativo
import os, shutil, struct, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, wait_for_js
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico
sys.path.insert(0, str(ROOT / "tests_e2e"))
import synth

from lib import free_port; PORT = free_port()
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

D = Path(tempfile.mkdtemp(prefix="agilent_gc_e2e_"))
top = D / "serie"
agilent_sintetico.write_old(top, "gcsynth.D", 60, gc=True)
agilent_sintetico.write_old(top / "cut", "incompleta.D", 60, gc=True)
(top / "cut/incompleta.D/data.ms").unlink()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):      # the server answers before the page is asked for
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: srv.poll() is None or sys.exit("server did not start"); threading.Event().wait(0.1)
def names(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")
def tree(d): return {str(f.relative_to(d.parent)): f.read_bytes().hex() for f in sorted(d.rglob("*")) if f.is_file()}
DROP = """async tree => { const file = (rel, hex) => ({ isFile: true, isDirectory: false, name: rel.split("/").pop(), file: ok => ok(new File([Uint8Array.from(hex.match(/../g) || [], h => parseInt(h, 16))], rel.split("/").pop())) });
  const dir = (n, kids) => ({ isFile: false, isDirectory: true, name: n, createReader() { let i = 0; return { readEntries(ok) { const b = kids.slice(i, i + 1); i += 1; ok(b); } }; } });
  const top = Object.keys(tree)[0].split("/")[0], kids = [], acq = [];
  for (const [rel, hex] of Object.entries(tree)) (rel.split("/").length === 3 ? acq : kids).push(file(rel, hex));
  const entry = dir(top, [...kids, dir("AcqData", acq)]);
  const ev = new Event("drop", { bubbles: true, cancelable: true });
  Object.defineProperty(ev, "dataTransfer", { value: { files: [], items: [{ kind: "file", getAsFile: () => null, webkitGetAsEntry: () => entry }] } });
  document.getElementById("drop").dispatchEvent(ev); }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        def page():
            c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            return pg
        def loaded(pg):
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            assert names(pg) == ["gcsynth"], names(pg)
            assert "has no AcqData" not in pg.inner_text("#up"), pg.inner_text("#up")
        def browse():
            pg = page(); pg.set_input_files("#pickdir", str(top / "gcsynth.D")); loaded(pg)
        step("browse: the GC .D (data.ms on top, AcqData with only Contents.xml) loads as one file, no notice", browse)
        def drop():
            pg = page(); pg.evaluate(DROP, tree(top / "gcsynth.D")); loaded(pg)
        step("drop: the same folder, dropped, loads as one file, no notice", drop)
        def notice():
            pg = page(); pg.set_input_files("#pickdir", str(top / "cut/incompleta.D"))
            pg.wait_for_function("document.querySelector('#up') && /has no AcqData/.test(document.querySelector('#up').innerText)", timeout=60000)
            t = pg.inner_text("#up")
            assert "AcqData/Contents.xml (" in t and "GC.ini (" in t and "data.ms" not in t.split("Received:")[-1], t
        step("a .D without data.ms: the notice lists what was received, with sizes", notice)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
for e in errs: print("  ", e)
sys.exit(1 if errs else 0)
