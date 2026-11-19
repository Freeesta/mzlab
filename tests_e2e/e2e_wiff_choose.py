"""«Choose samples» dialog of a .wiff with several samples (wiff-choose.js, docs/agenti/wiff-sample-choice.md PR 4): shown only when the file has several samples, all ticked for a small
set, Select none / numbers field (the engine's `sample_selection`), Open selected opens only the ticked ones, Cancel opens nothing, the setting «never» and «always». Synthetic pairs of
tools/wiff_grande.py. Needs the site and the .wasm: python tools/build_site.py, and the .wasm built here when cargo and wasm-bindgen exist."""
# covers: mzlab/web/wiff-choose, mzlab/web/settings, mzlab/web/shell, mzlab/web/rust-nativo, mzlab/web/rust-worker, crates/mzlab-core/src/readers/selection
import os, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, Run

def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:1500]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists() or not wasm.exists():
    print("  ", ("site", "FAIL site/ or its .wasm missing: python tools/build_site.py, then build mzlab-wasm")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="wiffchoose_e2e_"))
for name, n in (("multi", 3), ("solo", 1)):
    subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_grande.py"), str(D / name), "--name", name, "--samples", str(n), "--scans", "40", "--points", "50"], check=True, capture_output=True)
def pair(name): return [str(D / name / f"{name}.wiff"), str(D / name / f"{name}.wiff.scan")]
run = Run(); PORT = run.port
DLG = "dialog#wiffchoose[open]"
def rows(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=use]')).length")
def dlg(pg): pg.wait_for_selector(DLG, timeout=60000)
def ticked(pg): return pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose tbody input:checked')).length")
def settle(pg): pg.wait_for_function("!document.querySelector('dialog#wiffchoose')", timeout=30000)

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1300, "height": 900}, device_scale_factor=2, pick_dialog=True); c.set_default_timeout(120000); pg = c.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
        def key(): return pg.evaluate("NOMI.KEY('pick')")
        def several():
            pg.set_input_files("#pick", pair("multi")); dlg(pg)
            assert ticked(pg) == 3, ticked(pg)
            assert pg.get_attribute("#wiffchoose", "aria-labelledby") == "wc-title" and pg.evaluate("document.querySelector('#wiffchoose').matches(':modal')")
            assert pg.get_attribute("#wiffchoose .wc-sel", "role") == "status" and pg.evaluate("document.querySelectorAll('#wiffchoose tbody input[aria-label]').length") == 3
            assert pg.evaluate("Array.from(document.querySelectorAll('#wiffchoose button, #wiffchoose input')).every(e => e.getBoundingClientRect().height >= 24 && e.getBoundingClientRect().width >= 24)")
            shot = os.environ.get("MZLAB_SHOTS")
            if shot: Path(shot).mkdir(parents=True, exist_ok=True); pg.screenshot(path=str(Path(shot) / "wiff-choose-dialog.png"))
        step("several samples: the dialog opens with every sample ticked and is accessible", several)
        def none_then_numbers():
            pg.click("#wiffchoose button:text('Select none')"); assert ticked(pg) == 0 and pg.is_disabled("#wiffchoose button.go")
            pg.fill("#wc-ids", "2-3"); pg.click("#wiffchoose button:text('Tick numbers')"); pg.wait_for_function("document.querySelectorAll('#wiffchoose tbody input:checked').length === 2"); assert ticked(pg) == 2
            pg.fill("#wc-ids", "9"); pg.click("#wiffchoose button:text('Tick numbers')")
            pg.wait_for_function("document.querySelector('#wiffchoose .wc-msg').textContent.includes('no sample number 9')")
            pg.fill("#wc-ids", "x"); pg.click("#wiffchoose button:text('Tick numbers')"); pg.wait_for_function("document.querySelector('#wiffchoose .wc-msg').textContent.includes('Not a list')")
        step("Select none disables Open; numbers go through the engine (2-3 ticks two, 9 and x are told)", none_then_numbers)
        def cancel():
            pg.keyboard.press("Escape"); settle(pg); assert rows(pg) == 0, rows(pg)
        step("Escape cancels: nothing is opened", cancel)
        def open_some():
            pg.set_input_files("#pick", pair("multi")); dlg(pg)
            pg.click("#wiffchoose tbody tr:nth-child(2) input"); assert ticked(pg) == 2
            pg.click("#wiffchoose button.go"); settle(pg)
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
            assert rows(pg) == 2, rows(pg)
        step("Open selected opens only the ticked samples", open_some)
        def solo():
            n = rows(pg); pg.set_input_files("#pick", pair("solo"))
            pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length > {n}", timeout=120000)
            assert not pg.query_selector("dialog#wiffchoose")
        step("one sample per file: no dialog", solo)
        def never():
            pg.evaluate(f"localStorage.setItem({key()!r}, 'never')"); n = rows(pg)
            pg.set_input_files("#pick", pair("multi")); pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n} + 3", timeout=120000)
            assert not pg.query_selector("dialog#wiffchoose")
        step("setting «never»: every sample opens without asking", never)
        def always():
            pg.evaluate(f"localStorage.setItem({key()!r}, 'always')")
            pg.set_input_files("#pick", pair("solo")); dlg(pg); assert ticked(pg) == 1
            pg.click("#wiffchoose button:text('Cancel')"); settle(pg)
        step("setting «always»: a file with one sample is asked too", always)
        b.close()
finally:
    srv = run.srv; srv.terminate(); import shutil; shutil.rmtree(D, ignore_errors=True)
