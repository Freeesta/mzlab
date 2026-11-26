"""The scan list of the high-resolution bench shows the name/precursor warning of `file_method/1` (docs/agenti/method.md) with its numbers: a synthetic .wiff pair
(tools/wiff_riferimento.py `dispersione`: product ion 253.1, a sample named for 237.1) is opened, the file is put in the bench with a high-resolution profile (the
synthetic pair is a triple-quad file) and the «Scans» tab of the information bar lists the scans AND the warning, which blocks nothing. Needs the site and the .wasm
(python tools/build_site.py)."""
# covers: mzlab/web/banco.js, mzlab/web/metodo-file.js
import shutil, subprocess, sys, tempfile
from pathlib import Path
from lib import *
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ (with static/wasm) not built: python tools/build_site.py")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="scanlist_namemz_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
R = Run()
try:
    with sync_playwright() as p:
        pg = R.page(p); pg.set_default_timeout(120000); pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{R.port}/")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
        pg.set_input_files("#pick", [str(D / "dispersione.wiff"), str(D / "dispersione.wiff.scan")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
        pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=use]')).forEach(i => { if (!i.checked) i.click(); })")
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
        # the pair is a triple-quad file: the bench (and its scan list) exists only for a high-resolution profile, so the profile is set on the open file
        pg.evaluate("E.files.forEach(f => { f.prof1 = f.prof2 = { hr: true, an: 'orbitrap', dec: 4, tol: 5, unit: 'ppm' }; }); BANCO.bar(); BANCO.renderInfo()")
        def lst():
            pg.evaluate("setActive(E.panels.find(p => p.type === 'chrom'))")
            pg.evaluate("BARRA.setTab('lst')")
            pg.wait_for_selector("#hri-list .r", timeout=60000)
        step("the scan list opens for the sample", lst)
        def warning():
            pg.wait_for_selector("#hri-body [data-mfile=namewarn]", timeout=60000)
            t = pg.inner_text("#hri-body [data-mfile=namewarn]")
            for want in ("237.1", "253.1", "16"):
                assert want in t, (want, t)
            assert pg.locator("#hri-list .r").count() >= 1 and pg.is_visible("#hri-list .r"), "the list is still there and readable"
        step("the warning shows the name mass, the nearest precursor and the difference, beside the list", warning)
        def noerr():
            assert not errs, errs
        step("no page error", noerr)
finally:
    R.close()
    shutil.rmtree(D, ignore_errors=True)
