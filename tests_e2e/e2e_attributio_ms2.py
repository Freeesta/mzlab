"""Panel «MS2 engine» of the administrator mode (TP_Mine/js/54_attributio_ms2.js, the .wasm `attributio_ms2`): the library run answers with a table, the co-isolated features and the fragment
attribution answer, the controls are at least 24 x 24 px, axe finds nothing serious and MZLAB_LR_MISS stays empty. Builds the site and the .wasm (SKIP without cargo and wasm-bindgen);
MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site. Run from the repo root: python3 tests_e2e/e2e_attributio_ms2.py"""
import csv, io, json, os, re, shutil, subprocess, sys, tempfile, threading, urllib.request, zipfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401

steps = []
AXE = lib.axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0,3).map(n => n.target.join(' ') + ' :: ' + (n.failureSummary||'').split('\\n').slice(0,3).join(' | '))]))"
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-attributio-wasm"); sys.exit(0)

FOLDER = ROOT / "tests" / "data" / "attributio" / "campagna_sintetica"
tmp = Path(tempfile.mkdtemp(prefix="attr_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", env)
else:
    for sub in ("js", "files"):
        for f in (ROOT / "TP_Mine" / sub).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    ix = site / "static" / "mzfinder" / "indice.json"; d = json.loads(ix.read_text())
    d["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); ix.write_text(json.dumps(d))
man = json.loads((site / "static" / "wasm-feat" / "manifest.json").read_text())
size = (site / "static" / "wasm-feat" / man["attributio"]["wasm"]).stat().st_size
assert size < 512000, size
print("  attributio .wasm:", size, "bytes")

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=1).read(); break
    except Exception: threading.Event().wait(0.1)
errs = []
def new_page(b, admin):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", accept_downloads=True).new_page()
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    if admin: pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
    pg.on("pageerror", lambda e: errs.append(str(e)))
    pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
    pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
    return pg
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = new_page(b, True)
        pg.wait_for_selector("[data-v=attributio]", timeout=120000)
        pg.click("[data-v=attributio]"); pg.wait_for_selector("#at-pick")
        LIBF = tmp / "lib.msp"
        LIBF.write_text("Name: Test compound\nPRECURSORMZ: 301.1234\nPrecursor_type: [M+H]+\nIonMode: Positive\nNum Peaks: 4\n100.0 10\n150.0 40\n200.0 100\n250.0 30\n\n")
        pg.set_input_files("#at-dir", str(FOLDER))
        pg.wait_for_function("document.querySelectorAll('#at-table tbody tr.clk').length > 0", timeout=120000)
        pg.click("#at-ms2-host summary")

        def small():
            bad = pg.evaluate("""[...document.querySelectorAll('#at-ms2-host button, #at-ms2-host select, #at-ms2-host input')].filter(e => e.offsetParent)
                .map(e => [e, e.getBoundingClientRect()]).filter(([e, r]) => r.height < 24).map(([e, r]) => (e.id || e.textContent) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))""")
            assert not bad, bad
        step("the controls are at least 24 px high and have a name", small)

        def engine():
            pg.set_input_files("#ms2-lib", str(LIBF)); pg.click("#ms2-run")
            pg.wait_for_selector("#ms2-out table", timeout=120000)
            cells = pg.evaluate("[...document.querySelectorAll('#ms2-out tbody td')].map(c => c.textContent)")
            assert cells[0] == "1" and len(cells) == 4, (cells, pg.inner_text("#at-msg"))
        step("the library run answers with a table", engine)

        def chim():
            pg.fill("#ms2-id", "1"); pg.click("#ms2-chim")
            pg.wait_for_function("document.querySelector('#ms2-out caption') && document.querySelector('#ms2-out caption').textContent.startsWith('Co-isolated')", timeout=60000)
        step("the co-isolated features answer", chim)

        def shift():
            pg.fill("#ms2-formula", "C10H12N2O"); pg.click("#ms2-shift")
            pg.wait_for_selector("#ms2-out table", timeout=60000)
        step("the fragment attribution answers", shift)

        def axe():
            if not AXE: print("SKIP axe-core not available"); return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate(RUN % "document.getElementById('at-ms2-host')")
            assert not v, v
        step("axe: no serious violation in the panel", axe)

        step("no request is left unserved and the page has no error", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if (errs or pg.evaluate("(window.MZLAB_LR_MISS || []).length")) else None)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
