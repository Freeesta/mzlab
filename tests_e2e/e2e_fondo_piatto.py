"""Flat-window background of a chromatogram (fondo-piatto.js, Rust chrom_fondo.rs through qualita.js) on the built site: the program picks the window, the student moves it
(arrows, handle) and goes back to the automatic one. Needs the site and the .wasm (python tools/build_site.py; the CI builds the .wasm in site/static/wasm)."""
import subprocess, sys, time, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

from lib import free_port; PORT = free_port()
from synth import make_subtraction
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
site = ROOT / "site"
if not (site / "index.html").exists() or not (site / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "SKIP site/ or site/static/wasm not built")); sys.exit(0)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
PANEL = ".pnl.chrom"
CP = "E.panels.find(p=>p.type==='chrom')"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None); pg = b.new_context(viewport={"width": 1500, "height": 1200}, locale="it-IT").new_page()
        errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        files = make_subtraction("/tmp/wd_fondopiatto_files", blank=False)
        pg.set_input_files("#pick", [str(f) for f in files][:1])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
        def engine(): assert pg.evaluate("window.QUALITA ? QUALITA.flatWindow([5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,9,9,9]).then(w => !!w && w.i1 - w.i0 === 10) : false"), "the flat-window engine does not answer"
        step("the Rust flat-window engine answers", engine)
        def state(): return pg.evaluate(f"(()=>{{const ib={CP}.fondo.ib; return ib && {{a: ib.a, flat: !!ib.flat, mod: !!ib.mod}}}})()")
        def auto():
            pg.evaluate(f"setFondo({CP}, 'flat')"); pg.wait_for_function(f"(()=>{{const ib={CP}.fondo.ib; return ib && ib.flat && ib.a[1] > ib.a[0]}})()", timeout=30000); ready(pg)
            s = state(); assert s["flat"] and not s["mod"], s
            t = pg.evaluate(f"{CP}.el.querySelector('.ibk').textContent"); assert "finestra piatta" in t and "automatica" in t, t
            assert "automatica" in pg.evaluate(f"{CP}.el.querySelector('.ibk').getAttribute('aria-label')")
            globals()["A0"] = s["a"]
        step("the flat window is found by the program and the chip says «automatica»", auto)
        def keyboard():
            pg.locator(f"{PANEL} [data-o=cinfo]").click(); pg.wait_for_selector(f"{PANEL} .corrbar [data-ib='fl+']", state="visible")      # the chip lives in the «Correzioni» popover
            pg.locator(f"[data-ib='fl+']").first.focus(); pg.keyboard.press("ArrowRight"); ready(pg)
            pg.wait_for_function(f"{CP}.fondo.ib.mod === true", timeout=10000); ready(pg)
            s = state(); assert s["a"][0] > A0[0] and s["mod"], (s, A0)
            t = pg.evaluate(f"{CP}.el.querySelector('.ibk').textContent"); assert "modificata da te" in t and "ripristina automatico" in t, t
            ae = pg.evaluate("document.activeElement && (document.activeElement.dataset.ib || document.activeElement.outerHTML.slice(0, 80))"); assert ae == "fl+", ("the focus must stay on the arrow", ae)
        step("one arrow key moves the window: «modificata da te» and the restore button appear, the focus stays", keyboard)
        def drag():
            pg.keyboard.press("Escape"); ready(pg)
            before = state()["a"][0]; box = pg.evaluate(f"(()=>{{const p={CP},a=p._a,r=p.cv.getBoundingClientRect(),ib=p.fondo.ib;return {{x:r.left+(a.X(ib.a[0])+a.X(ib.a[1]))/2,y:r.top+M.t+5,dx:60}}}})()")
            pg.mouse.move(box["x"], box["y"]); pg.mouse.down(); pg.mouse.move(box["x"] + box["dx"], box["y"], steps=6); pg.mouse.up(); ready(pg)
            assert abs(state()["a"][0] - before) > 1e-6 and state()["mod"], (before, state())
        step("dragging the handle on the chart moves the window", drag)
        def restore():
            pg.locator(f"{PANEL} [data-o=cinfo]").click(); pg.wait_for_selector(f"{PANEL} .corrbar [data-ib='fl0']", state="visible")
            pg.locator("[data-ib='fl0']").first.click(); ready(pg); pg.wait_for_function(f"{CP}.fondo.ib.mod !== true", timeout=10000); ready(pg)
            pg.wait_for_function(f"Math.abs({CP}.fondo.ib.a[0] - {A0[0]!r}) < 1e-9", timeout=30000)
            assert "automatica" in pg.evaluate(f"{CP}.el.querySelector('.ibk').textContent")
        step("«ripristina automatico» gives back the automatic window", restore)
        def clean(): assert not errs, errs[:3]
        step("no page error", clean)
        b.close()
finally:
    srv.terminate()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
