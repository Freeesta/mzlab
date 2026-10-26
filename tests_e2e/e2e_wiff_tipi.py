"""The experiment type that the list of the files shows for each sample of a .wiff with mixed experiments (synthetic, tools/wiff_misto.py; no laboratory data), on the built site with the
Rust engine (?motore=rust&lr=1): every sample has its own type (Full Scan, MS2 or MRM) in the state of the page and in the group of its row, not the one of the first sample or of the method.
Needs the site and the .wasm (python tools/build_site.py + wasm, see e2e_wiff.py). Run from the repo root: python3 tests_e2e/e2e_wiff_tipi.py"""
import json, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, attendi, free_port

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:900]))

if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py + wasm")); sys.exit(0)
D = Path(tempfile.mkdtemp(prefix="wiff_tipi_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_misto.py"), str(D)], check=True, capture_output=True)
TRUE = json.loads((D / "tipi.json").read_text())
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import urllib.request
for _ in range(50):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1); break
    except Exception: __import__("threading").Event().wait(0.2)      # the server is starting
SHOWN = """() => ST.files.map(f => [f.name, f.kind, (f.mixed || []).map(p => p.name).join('+'), (document.querySelector(`#flist tr[data-n="${CSS.escape(f.name)}"]`) || {}).innerText || ''])"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop"); attendi(pg, "window.MZLAB_LR !== undefined && !!document.getElementById('pick').onchange")
        pg.set_input_files("#pick", [str(D / "misto.wiff"), str(D / "misto.wiff.scan")])
        attendi(pg, f"ST.files.length === {len(TRUE)} && ST.files.every(f => f.kind)", timeout=120000)
        got = pg.evaluate(SHOWN)
        print("   shown:", json.dumps([g[:3] for g in got]))
        def kinds():
            by = {i + 1: g for i, g in enumerate(sorted(got, key=lambda g: int(g[0].rsplit("_s", 1)[1].split(".")[0]) if "_s" in g[0] else 0))}
            bad = {n: (TRUE[str(n)], by[n][1], by[n][2]) for n in by if by[n][1] != TRUE[str(n)] or by[n][2]}
            assert not bad, bad
        step("each sample of the .wiff has its own experiment type in the list", kinds)
        step("no page error", lambda: errs and (_ for _ in ()).throw(AssertionError(errs)))
        b.close()
finally:
    srv.terminate()
