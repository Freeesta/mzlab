"""A SCIEX .wiff or .wiff.scan picked ALONE: the page asks for the folder, takes only the companion from it and opens the pair (wiff-pair.js).
The folder dialog is a stub (a native dialog cannot be driven): a fake folder handle with decoy files whose reading would be recorded. Cases: only the .wiff, only the
.wiff.scan (the companion in capitals), both picked (no prompt), companion absent (the message names the file and the folder). Synthetic pair of tools/wiff_riferimento.py.
Needs the site and the .wasm: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist."""
# covers: mzlab/web/wiff-pair, mzlab/web/drop-unico, mzlab/web/shell, crates/mzlab-lettori/src/pair
import base64, shutil, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, Run

def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:1500]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists() or not wasm.exists():
    print("  ", ("site", "FAIL site/ or its .wasm missing: python tools/build_site.py, then build mzlab-wasm")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="wiffpair_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
for n in ("a1", "a2", "a3", "a4"):
    for x in ("wiff", "wiff.scan"): shutil.copy(D / f"one.{x}", D / f"{n}.{x}")
b64 = lambda p: base64.b64encode(p.read_bytes()).decode()
run = Run(); PORT = run.port; srv = run.srv
STUB = """(() => {
  window.__read = []; window.__opts = null; window.__next = null;
  const file = (n, b64) => ({ kind: 'file', name: n, getFile: async () => { window.__read.push(n); const s = atob(b64), a = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) a[i] = s.charCodeAt(i); return new File([a], n); } });
  window.__dir = (name, items) => { window.__next = { name, kind: 'directory', async *entries() { for (const [n, b] of items) yield [n, file(n, b)]; } }; };
  window.showDirectoryPicker = async o => { window.__opts = o; return window.__next; };
})()"""
JUNK = base64.b64encode(b"decoy").decode()
def names(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")
def shown(pg, n): pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=120000)
def prompt(pg): pg.wait_for_selector("#wiffpair:not([hidden]) button[data-wp=choose]", timeout=60000); return pg.inner_text("#wiffpair")

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1300, "height": 900}, device_scale_factor=2); c.set_default_timeout(120000); pg = c.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.add_init_script(STUB)
        pg.goto(f"http://127.0.0.1:{PORT}/")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange", timeout=60000)
        def only_wiff():
            pg.evaluate("DROP.handles.set('a1.wiff', { name: 'a1.wiff', kind: 'file' })")
            pg.set_input_files("#pick", [str(D / "a1.wiff")])
            t = prompt(pg); assert "a1.wiff.scan is needed" in t, t
            assert pg.get_attribute("#wiffpair", "role") == "status" and pg.get_attribute("#wiffpair", "aria-live") == "polite"
            assert len(pg.get_attribute("#wiffpair button[data-wp=choose]", "title")) <= 60
            shot = __import__("os").environ.get("MZLAB_SHOTS")
            if shot: Path(shot).mkdir(parents=True, exist_ok=True); pg.screenshot(path=str(Path(shot) / "wiff-pair-prompt.png"))
            pg.evaluate("window.__dir('Lab folder', [['other.mzML', %r], ['a1.wiff', %r], ['a1.wiff.scan', %r], ['notes.txt', %r]])" % (JUNK, JUNK, b64(D / "a1.wiff.scan"), JUNK))
            pg.click("#wiffpair button[data-wp=choose]")
            shown(pg, 1); assert names(pg) == ["a1"], names(pg)
            assert pg.evaluate("window.__opts.mode") == "read" and pg.evaluate("window.__opts.startIn.name") == "a1.wiff", pg.evaluate("window.__opts")
            assert pg.evaluate("window.__read") == ["a1.wiff.scan"], pg.evaluate("window.__read")
            s = pg.inner_text("#wiffpair"); assert "Found: a1.wiff.scan" in s and "opening the pair" in s, s
        step("only the .wiff: prompt, dialog starts at its handle, only the .wiff.scan is read, the pair opens", only_wiff)
        def only_scan():
            pg.evaluate("window.__read.length = 0")
            pg.set_input_files("#pick", [str(D / "a2.wiff.scan")])
            t = prompt(pg); assert "a2.wiff is needed" in t, t
            pg.evaluate("window.__dir('Lab folder', [['A2.WIFF', %r], ['x.wiff', %r]])" % (b64(D / "a2.wiff"), JUNK))
            pg.click("#wiffpair button[data-wp=choose]")
            shown(pg, 2); assert sorted(n.lower() for n in names(pg)) == ["a1", "a2"], names(pg)
            assert pg.evaluate("window.__read") == ["A2.WIFF"], pg.evaluate("window.__read")
        step("only the .wiff.scan: the companion in capitals is found by name", only_scan)
        def both():
            pg.evaluate("document.getElementById('wiffpair') && (document.getElementById('wiffpair').hidden = true)")
            pg.set_input_files("#pick", [str(D / "a3.wiff"), str(D / "a3.wiff.scan")])
            shown(pg, 3); assert "a3" in names(pg), names(pg)
            assert pg.evaluate("!document.getElementById('wiffpair') || document.getElementById('wiffpair').hidden")
        step("both picked: no prompt", both)
        def missing():
            pg.evaluate("window.__read.length = 0")
            pg.set_input_files("#pick", [str(D / "a4.wiff")])
            prompt(pg)
            pg.evaluate("window.__dir('Empty folder', [['a5.wiff.scan', %r], ['a4.mzML', %r]])" % (JUNK, JUNK))
            pg.click("#wiffpair button[data-wp=choose]")
            pg.wait_for_function("document.getElementById('wiffpair').innerText.includes('Not found')", timeout=30000)
            t = pg.inner_text("#wiffpair"); assert "a4.wiff.scan" in t and "Empty folder" in t, t
            assert pg.evaluate("window.__read") == [] and "a4" not in names(pg), names(pg)
            pg.click("#wiffpair button[data-wp=skip]"); assert pg.evaluate("document.getElementById('wiffpair').hidden")
        step("companion absent: the message names the file and the folder, nothing else is read, «Not now» closes it", missing)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(D, ignore_errors=True)
