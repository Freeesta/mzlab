"""The Method window shows `file_method/1` (docs/agenti/method.md) on the built site: a synthetic .wiff pair (tools/wiff_riferimento.py `dispersione`: product ion 253.1, CE 35,
CES 15, DP 60, a sample named for 237.1) is opened; /api/filemethod gives the document, the window lists the segment with its values (two decimals), marks fill time and Q1 resolution n/r with a legend and the reason as tooltip and aria-label and shows the name/precursor warning with its numbers, and nothing is blocked. Needs the site and the .wasm (python tools/build_site.py)."""
# covers: mzlab/web/metodo-file.js, mzlab/web/metodo-doc.js, mzlab/web/lr-method.js
import shutil, subprocess, sys, tempfile
from pathlib import Path
from lib import *
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ (with static/wasm) not built: python tools/build_site.py")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="metodo_file_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
R = Run()
JSON = """async u => { const r = await fetch(u); return { status: r.status, j: await r.json() }; }"""

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
        def route():
            r = pg.evaluate(JSON, "api/filemethod?k=0")
            assert r["status"] == 200 and r["j"]["schema"] == "file_method/1", r
            s = next(x for x in r["j"]["segments"] if x["ms_level"] == 2)
            assert (s["precursor_mz"], s["ce_eV"], s["ces_eV"], s["dp_V"]) == (253.1, 35, 15, 60), s
            assert r["j"]["unavailable"] == {"fill_time_ms": "unknown", "q1_resolution": "unknown"}, r["j"]["unavailable"]
        step("/api/filemethod gives file_method/1 of the .wiff sample", route)
        def window():
            pg.click("#np-method")
            pg.wait_for_selector("#bigdlg [data-mfile=segments]", timeout=60000)
            t = pg.inner_text("#bigdlg [data-mfile=segments]")
            for want in ("253.10", "35.00 eV", "15.00 eV", "60.00 V", "n/r"):
                assert want in t, (want, t)
            assert "not read for this format" not in t, t
            assert "n/r = not read by mzLab for this format" in pg.inner_text("#bigdlg [data-mfile=legend]")
            long = pg.get_attribute("#bigdlg [data-mfile=segments] td[aria-label]:has-text('n/r')", "aria-label")
            assert long and "not read" in long.lower(), long
        step("the Method window lists precursor, CE, CES, DP and the reason fill time and Q1 resolution are missing", window)
        def warning():
            t = pg.inner_text("#bigdlg [data-mfile=notes]")
            assert "237.10" in t and "253.10" in t and "16.00" in t, t
            pg.keyboard.press("Escape")
        step("the name/precursor warning shows its numbers and blocks nothing", warning)
        def noerr():
            assert not errs, errs
        step("no page error", noerr)
finally:
    R.close()
    shutil.rmtree(D, ignore_errors=True)
