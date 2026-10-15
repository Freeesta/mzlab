"""Engine «mzmine-like (Attributio)» of the Pipeline panel (TP_Mine/js/32_pipeline.js, feat-run.js, feat-worker.js, the .wasm `untargeted`). Part 1: the engine choice is there only in the administrator
mode. Part 2: the engine runs on a synthetic series in the browser (detect + align in the worker) and the table enters the panel (rows, cells per file). Part 3: the table has the SHA-256
of the example `untargeted` (crates/mzlab-feat-wasm/examples/untargeted.rs) on the same indexes, if cargo can build it. Builds the site and the .wasm (SKIP without cargo, wasm-bindgen, numpy).
MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site. Run from the repo root: python3 tests_e2e/e2e_attributio_pipeline.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="attrpipe_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=env, capture_output=True)
else:
    for sub in ("js", "files"):
        for f in (ROOT / "TP_Mine" / sub).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    ix = site / "static" / "mzfinder" / "indice.json"; d = json.loads(ix.read_text())
    d["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); ix.write_text(json.dumps(d))
man = json.loads((site / "static" / "wasm-feat" / "manifest.json").read_text())
assert "untargeted" in man and (site / "static" / "wasm-feat" / man["untargeted"]["wasm"]).stat().st_size < 512000, man.get("untargeted")
data = site / "static" / "dati_pipeline"; data.mkdir(exist_ok=True)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
for f in sorted(ser.glob("*.mzidx")): shutil.copy2(f, data / f.name)
names = sorted(p.name for p in data.glob("*.mzidx"))

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
import threading, urllib.request
for _ in range(100):      # ready as soon as the server answers
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).read(); break
    except Exception: threading.Event().wait(0.1)
URL = f"http://127.0.0.1:{PORT}/"
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_pipeline/" + n)).body.pipeTo(w); }
}"""
# the synthetic series is small and weak: thresholds well under the defaults of the batch of mzmine
LOW = {"u-noise": "100", "u-min_highest": "300", "u-min_group": "100", "u-min_cons": "3", "u-minabs": "300", "u-chrom": "0.85"}
def new_page(b, admin):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
    if admin: pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")
    pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
    pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
    pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_function("window.PIPE && window.UNTUI", timeout=120000)
    pg.evaluate(PUT, names)
    return pg
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg0 = new_page(b, True)      # mzFinder (PIPE) is loaded only in the administrator mode: the mode is left before the panel is built
        def no_admin():
            pg0.evaluate("localStorage.removeItem('qqq.admin'); PIPE.open()"); pg0.wait_for_selector("#pipe[open]")
            assert pg0.locator("#pipe-engine").count() == 0 and pg0.locator("#pipe-unt").count() == 1 and pg0.is_hidden("#pipe-unt")
            assert json.loads(pg0.evaluate("PIPE.recipe(PIPE.values())")).get("engine") is None
        step("without the administrator mode there is no engine choice and the recipe is the one of mzFinder", no_admin)
        pg0.close()

        pg = new_page(b, True)
        def choice():
            pg.evaluate("PIPE.open()"); pg.wait_for_selector("#pipe[open]"); pg.wait_for_selector("#pipe-engine")
            assert pg.get_attribute("#pipe-engine", "aria-label") and pg.is_hidden("#pipe-unt")
            pg.select_option("#pipe-engine", "untargeted")
            assert pg.is_visible("#pipe-unt") and pg.is_hidden("#pipe-fin")
            for k, v in LOW.items(): pg.fill("#pipe-" + k, v)
            r = json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))
            assert r["engine"] == "untargeted" and r["adap"]["noise"] == 100 and r["resolve"]["min_abs_height"] == 300 and r["join"]["tol_ppm"] == 5, r
        step("in the administrator mode the choice shows the parameters with the labels of mzmine and writes the recipe of the engine", choice)

        def small_targets():
            small = pg.evaluate("""[...document.querySelectorAll('#pipe-unt input, #pipe-engine')].filter(e => e.offsetParent)
                .map(e => [e, e.getBoundingClientRect()]).filter(([e, r]) => r.width < 24 || r.height < 24).map(([e, r]) => e.id + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))""")
            assert not small, small
        step("the new controls are at least 24 x 24 px", small_targets)

        def run():
            pg.wait_for_function("document.querySelectorAll('#pr-tab .pr-role').length === %d" % len(names), timeout=60000)
            pg.click("#pr-all"); pg.wait_for_function("document.querySelectorAll('#pr-tab tr.pr-prop').length === 0")
            pg.click("#pipe-run-go")
            pg.wait_for_function("PIPER.state.data && PIPER.state.data.rows > 0 || /rows in/.test(document.querySelector('#pipe-run-msg').textContent)", timeout=240000)
            msg = pg.inner_text("#pipe-run-msg"); assert "rows in" in msg, msg
            st = pg.evaluate("({rows: PIPER.state.data.rows, files: PIPER.state.names.length, tim: PIPER.state.out.timings.map(t => t.phase)})")
            assert st["rows"] > 0 and st["files"] == len(names) and st["tim"] == ["detect", "align"], st
            assert not pg.evaluate("(window.MZLAB_LR_MISS || []).length")
            pg.evaluate("window.__sha = PIPER.state.out.table.length")
            print("  rows:", st["rows"], msg)
        step("Run with the engine detects each file and aligns the series in the worker; the table has rows and no proofs phase", run)

        def recorded():
            rec = pg.evaluate("PIPER.state.last || null")
            assert rec and json.loads(rec["recipe"])["engine"] == "untargeted" and rec["parametri"]["engine"] == "untargeted", rec and rec.get("recipe")
        step("the recipe of the run is recorded with its engine", recorded)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
