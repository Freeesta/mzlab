"""Loading several samples that need the readers module (rule R of shell.js upload0): the first sample is read and listed as soon as its own conversion ends, while the next ones are still converting
(moduloToMzml hands each file to upload0 as it is ready). Three synthetic Agilent .D folders (tools/agilent_sintetico.py, no laboratory data), the first one small and the other two big, with
?lettori=modulo: the page notes when each mzML is written (CONV) and when the first one is in the list (ST.files). Needs the site and both .wasm like e2e_lettori_modulo
(MZLAB_PIPE_SITE=<built site> reuses a site). Run from the repo root: python3 tests_e2e/e2e_apertura_pipeline.py"""
import os, shutil, socket, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import agilent_sintetico

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp(prefix="pipeline_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    src = ROOT / "site" / "static" / "wasm-lettori"
    assert (src / "manifest.json").exists(), "build it: tools/build_lettori_wasm.sh site/static"
    shutil.copytree(src, site / "static" / "wasm-lettori", dirs_exist_ok=True)
top = tmp / "serie"
for name, n in (("a1.D", 60), ("b2.D", 40000), ("c3.D", 40000)):   # the first converts in a blink, the other two take long
    agilent_sintetico.write(top, name, n)

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):   # the server answers
    try: socket.create_connection(("127.0.0.1", PORT), 1).close(); break
    except OSError: threading.Event().wait(0.1)
errs = []
# the page notes, every 10 ms, the time at which the first sample is in the list and the time at which the last mzML is written
WATCH = """() => { window.__t = {}; const t0 = performance.now();
  setInterval(() => { const T = window.__t;
    if (!T.first && typeof ST !== "undefined" && ST.files.some(x => x.name === "a1.mzML")) T.first = performance.now() - t0;
    if (!T.last && typeof CONV !== "undefined" && CONV.files.has("c3.mzML")) T.last = performance.now() - t0; }, 10); }"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1&lettori=modulo"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
        pg.evaluate(WATCH)
        pg.set_input_files("#pickdir", str(top))
        pg.wait_for_function("window.__t.last", timeout=300000)
        pg.wait_for_function("ST.files.length >= 3", timeout=300000)
        t = pg.evaluate("window.__t")
        def first_before_last():
            assert t.get("first") and t.get("last"), t
            assert t["first"] < t["last"], f"the first sample was listed at {t['first']:.0f} ms, after the last conversion ended ({t['last']:.0f} ms)"
        step("the first sample is in the list before the last one has finished converting", first_before_last)
        def all_open():
            names = sorted(pg.evaluate("ST.files.map(x => x.name)"))
            assert names == ["a1.mzML", "b2.mzML", "c3.mzML"], names
            assert (pg.evaluate("window.MZLAB_LR_MISS || []")) == [], "MZLAB_LR_MISS is empty"
        step("all three samples end up in the list and nothing misses the engine", all_open)
        b.close()
finally:
    srv.terminate()
for n, r in steps: print(r[:2] if r == "ok" else r, n)
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
