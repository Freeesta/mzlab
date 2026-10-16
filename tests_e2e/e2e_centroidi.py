"""«Riduci ogni picco a un solo punto» (Impostazioni): a low-resolution profile mzML gets a copy `<name>.centroidi.mzML` loaded next to it, the original stays as it is; the card says
«Profilo» / «Un punto per picco»; the status shows «N di M». On the built site with the Rust engine and WITHOUT Pyodide (?motore=rust&lr=1); a synthetic file (no laboratory data).
Needs the site and the .wasm: python tools/build_site.py (the .wasm is built as in e2e_wiff.py)."""
import shutil, subprocess, sys, tempfile, time
from pathlib import Path
import numpy as np
from playwright.sync_api import sync_playwright
from lib import ROOT
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds

from lib import free_port; PORT = free_port()
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("wasm", "FAIL site/static/wasm missing")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="cen_e2e_"))
rng = np.random.default_rng(3)
specs = []
for i in range(60):
    x = 297.0 + 0.06 * np.arange(200)
    lam = 0.3 + sum(h * np.exp(-(x - c) ** 2 / (2 * (0.7 / 2.355) ** 2)) for c, h in ((300.0, 900), (304.0, 700), (306.0, 500)))
    n = rng.poisson(lam); k = n > 0
    specs.append(ds._spec(i, 0.1 * i, x[k], n[k] * 2.5, profile=True))
ds._write(D / "prof.mzML", specs, [ds._chrom(0, "TIC", np.arange(60) * 0.1, np.ones(60))])
size0 = (D / "prof.mzML").stat().st_size

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1200}); c.set_default_timeout(120000); pg = c.new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.set_input_files("#pick", [str(D / "prof.mzML")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
        def card_profile():
            pg.wait_for_function("typeof ST !== 'undefined' && ST.files.length === 1 && ST.files[0].spectrum_mode === 'profile'", timeout=60000)
            t = pg.evaluate("modeTag({ kind: 'full', spectrum_mode: 'profile' })")
            assert "Profilo" in t, t
        step("the card of a profile file says Profilo", card_profile)
        def convert():
            pg.click("#np-set"); pg.wait_for_selector("#uip-cen")
            assert "Centroidazione" in pg.get_attribute("#uip-cen", "title")
            pg.click("#uip-cen"); pg.click("#askok")
            pg.wait_for_function("typeof ST !== 'undefined' && ST.files.length === 2", timeout=60000)
            names = sorted(f["name"] for f in pg.evaluate("ST.files"))
            assert names == ["prof.centroidi.mzML", "prof.mzML"], names
        step("the command makes prof.centroidi.mzML and loads it", convert)
        def modes():
            m = pg.evaluate("Object.fromEntries(ST.files.map(f => [f.name, f.spectrum_mode]))")
            assert m == {"prof.mzML": "profile", "prof.centroidi.mzML": "centroid"}, m
            t = pg.evaluate("modeTag({ kind: 'full', spectrum_mode: 'centroid' })")
            assert t == "" and "Un punto per picco" not in pg.inner_text("#flst"), t      # the usual centroid file carries no label
        step("the copy is a centroid file (no label), the original is still a profile file", modes)
        def copy_text():
            t = pg.evaluate("""async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files'); const f = await (await d.getFileHandle('prof.centroidi.mzML')).getFile(); return [f.size, await f.text()]; }""")
            assert "MS:1000035" in t[1] and "mzlab-centroidi" in t[1] and t[0] < size0, (t[0], size0)
        step("the copy says it was centroided by mzLab and is smaller", copy_text)
        def original_kept():
            n = pg.evaluate("""async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files'); return (await (await d.getFileHandle('prof.mzML')).getFile()).size; }""")
            assert n == size0, (n, size0)
        step("the original file in the archive is not changed", original_kept)
        def again_nothing():
            pg.keyboard.press("Escape")
            pg.click("#np-set")
            assert pg.locator("#uip-cen").count() == 0, "a second run has nothing to do"
        step("with both files loaded the command has nothing left to do", again_nothing)
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs)) if errs else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(D, ignore_errors=True)
