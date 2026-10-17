"""metodo/1 on the built site with the Rust engine (?motore=rust&lr=1): an mzML that names a .wiff by SHA-1 (as ProteoWizard writes it) is opened, then that .wiff arrives ALONE
(no .wiff.scan): it is not an error, the mzML gets the method of its sample (notice + /api/metodo source "wiff"); a .wiff nobody names still gets the usual notice. Synthetic files only
(tools/wiff_riferimento.py, tools/dati_sintetici.py). Needs the site and the .wasm like e2e_wiff.py (python tools/build_site.py)."""
import hashlib, re, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

from lib import free_port; PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ (with static/wasm) not built: python tools/build_site.py")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="metodo_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
sha = hashlib.sha1((D / "one.wiff").read_bytes()).hexdigest()
ds.full_scan(D / "named.mzML", 0, np.random.default_rng(1))
t = (D / "named.mzML").read_text(encoding="utf-8")
t = t.replace("<instrumentConfigurationList", f'<fileDescription><fileContent/><sourceFileList count="1"><sourceFile id="SF1" name="one.wiff" location="file:///x"><cvParam cvRef="MS" accession="MS:1000569" name="SHA-1" value="{sha}"/></sourceFile></sourceFileList></fileDescription><instrumentConfigurationList', 1)
(D / "named.mzML").write_text(t, encoding="utf-8")
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, j: await r.json() }; }"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1200}); c.set_default_timeout(120000)
        pg = c.new_page(); pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.set_input_files("#pick", [str(D / "named.mzML")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
        pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=use]')).forEach(i => { if (!i.checked) i.click(); })")
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
        def none_yet():
            r = pg.evaluate(JSON, "api/metodo?k=0")
            assert r["status"] == 200 and r["j"]["source"] == "mzml", r
            m = r["j"]["metodo"]
            assert m["schema"] == "metodo/1" and m["origine"]["formato"] == "mzml", r
        step("an mzML without method: /api/metodo builds metodo/1 from its header", none_yet)
        def lone_with_match():
            pg.set_input_files("#pick", [str(D / "one.wiff")])
            pg.wait_for_function("fetch('api/metodo?k=0').then(r => r.json()).then(j => j.source === 'wiff')", timeout=60000)
            assert not (pg.locator("#unopen").count() and pg.locator("#unopen").is_visible() and "one.wiff" in pg.inner_text("#unopen")), pg.inner_text("#unopen")
        step("a lone .wiff named by the open mzML is no error", lone_with_match)
        def route():
            r = pg.evaluate(JSON, "api/metodo?k=0")
            assert r["status"] == 200 and r["j"]["source"] == "wiff", r
            m = r["j"]["metodo"]
            assert m["schema"] == "metodo/1" and m["origine"]["formato"] == "sciex-wiff" and m["origine"]["file"][0]["sha1"] == sha, m["origine"]
            assert m["origine"]["campione"] == 1, m["origine"]
        step("/api/metodo gives the metodo/1 of the .wiff sample", route)
        def window():
            pg.click("#np-method")
            pg.wait_for_function("document.querySelector('#bigdlg').innerText.includes('Metodo registrato nel file')", timeout=60000)
            t = pg.inner_text("#bigdlg")
            assert "sciex-wiff" in t and "SHA-1 " + sha in t, t
            pg.keyboard.press("Escape")
        step("the Method window shows the method stored in the file (metodo/1)", window)
        def lone_unnamed():
            pg.set_input_files("#pick", [str(D / "lonely.wiff")])
            pg.wait_for_function("document.querySelector('#unopen') && document.querySelector('#unopen').innerText.includes('lonely')", timeout=60000)
        step("a lone .wiff nobody names still gets the notice", lone_unnamed)
        def noerr():
            assert not errs, errs
        step("no page error", noerr)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
