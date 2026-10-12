"""Panel «Pipeline» of mzFinder (TP_Mine/js/32_pipeline.js), part 1: parameters with their proof, preview on one file, gate of quality (cancello.json).
Builds the site (python3 tools/build_site.py) and the two .wasm of the feature engine (tools/build_feat_wasm.sh), writes a synthetic .mzidx to OPFS, turns mzFinder on and
opens the panel. Needs cargo, the wasm32 target, wasm-bindgen and numpy (SKIP without). MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site (the TP_Mine
files are copied over it); MZLAB_PIPE_MZIDX=<file .mzidx> adds the time of the preview on a real file (under 5 s for 80 MB).
Run from the repo root: python3 tests_e2e/e2e_pipeline.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import axe_path

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

tmp = Path(tempfile.mkdtemp(prefix="pipeline_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=env, capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for sub in ("js", "files"):
        for f in (ROOT / "TP_Mine" / sub).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
data = site / "static" / "dati_pipeline"; data.mkdir(exist_ok=True)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
for f in sorted(ser.glob("*.mzidx"))[:2]: shutil.copy2(f, data / f.name)
real = os.environ.get("MZLAB_PIPE_MZIDX")
if real: shutil.copy2(real, data / "real.mzidx")
names = sorted(p.name for p in data.glob("*.mzidx"))

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
URL = f"http://127.0.0.1:{PORT}/"
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_pipeline/" + n)).body.pipeTo(w); }
}"""
GATE = {"format": "cancello/1", "crate_version": "", "data": "2026-10-12", "vista_candidati": True, "concordanza_mzmine": "passa", "criteri": [{"id": "c1", "valore": 1, "soglia": 1, "ok": True}]}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.PIPE && window.UNTUI", timeout=120000)
        pg.evaluate(PUT, names)

        def gate_absent():
            pg.evaluate("PIPE.open()"); pg.wait_for_selector("#pipe[open]")
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('not validated')", timeout=60000)
            assert pg.locator("#pipe-file option").count() >= 1
        step("without cancello.json the panel says «not validated»", gate_absent)

        def gate_states():
            crate = pg.evaluate("PIPE.state.crate"); assert crate, crate
            g = lambda c, cr: pg.evaluate("([c, cr]) => PIPE.gateState(c, cr)", [c, cr])
            ok = dict(GATE, crate_version=crate)
            assert g(ok, crate)["ok"] is True
            assert g(dict(ok, crate_version="0.0.0"), crate)["why"] == "stale"
            assert g(dict(ok, vista_candidati=False), crate)["why"] == "closed"
            assert g(dict(ok, format="x/9"), crate)["why"] == "format"
            assert g(None, crate)["why"] == "absent"
        step("gate: open only with vista_candidati and the crate version of the .wasm", gate_states)

        def gate_page():
            crate = pg.evaluate("PIPE.state.crate")
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version=crate))
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('candidate view is available')", timeout=60000)
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", dict(GATE, crate_version="0.0.0"))
            pg.wait_for_function("document.querySelector('#pipe-gate').textContent.includes('this version of the engine')", timeout=60000)
            pg.evaluate("delete QTOOLS.ctx.files['cancello.json']; PIPE.open();")
        step("gate in the panel: valid, other version of the engine", gate_page)

        def preview():
            pg.select_option("#pipe-file", names[0]); t0 = time.time()
            pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.includes('features in')", timeout=90000)
            n = pg.inner_text("#pipe-n"); assert "0 features" not in n, n
            assert pg.locator("#pipe-hist svg").count() == 2 and pg.locator("#pipe-ex svg").count() == 3, "charts"
            floor = pg.evaluate("PIPE.values().floor"); assert 1e4 < floor < 1e6, floor
            proof = pg.inner_text("#pipe-proof"); assert "Noise floor" in proof and "Mass tolerance" in proof and "Median peak width" in proof, proof
            print("  preview of", names[0], round(time.time() - t0, 2), "s;", n)
        step("preview: features, estimated floor, tolerance and width with their proof, 2 histograms and 3 XIC", preview)

        def tables():
            svgs = pg.locator("#pipe svg[role=img]"); assert svgs.count() == 5
            for i in range(5):
                el = svgs.nth(i); el.focus(); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
                assert pg.locator("#tdati tbody tr").count() >= 3, i
                pg.keyboard.press("Escape"); pg.wait_for_selector("#tdati", state="detached")
            assert pg.locator("#pipe .tdbtn").count() == 5
        step("every chart has its table of data (T key and Data button)", tables)

        def edit_and_again():
            pg.fill("#pipe-floor", "5e4"); assert "chosen" in pg.inner_text("#pipe-floor-why")
            assert json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))["floor"] == 5e4
            pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-msg').textContent === ''", timeout=60000)
            pg.wait_for_timeout(500)
            assert float(pg.evaluate("PIPE.values().floor")) == 5e4 and pg.evaluate("PIPE.state.prev.n") > 0
        step("a chosen floor is kept and sent to the engine", edit_and_again)

        def keyboard_and_targets():
            small = pg.evaluate("""[...document.querySelectorAll('#pipe button, #pipe select, #pipe input, #pipe svg[tabindex]')].filter(e => e.offsetParent)
                .map(e => [e, (e.closest('label') && e.matches('input[type=checkbox]') ? e.closest('label') : e).getBoundingClientRect()]).filter(([e, r]) => r.width < 24 || r.height < 24)
                .map(([e, r]) => (e.id || e.tagName) + ' ' + Math.round(r.width) + 'x' + Math.round(r.height))""")
            assert not small, small
        step("WCAG 2.5.8: the controls of the panel are at least 24 x 24 px", keyboard_and_targets)

        AXE = axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""axe.run('#pipe', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length]))""")
            assert not v, v
        step("axe: the panel has no violation", axe) if AXE else print("  SKIP axe-core not available")

        def shots():
            out = os.environ.get("MZLAB_PIPE_SHOTS")
            if not out: return
            for w in (1280, 390):
                pg.set_viewport_size({"width": w, "height": 1400}); pg.wait_for_timeout(300)
                pg.locator("#pipe").screenshot(path=str(Path(out) / f"pipeline_{w}.png"))
        shots()

        if real:
            def real_file():
                pg.select_option("#pipe-file", "real.mzidx"); t0 = time.time()
                pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.includes('real.mzidx')", timeout=120000)
                dt = time.time() - t0; print("  preview of the real file:", round(dt, 2), "s"); assert dt < 5, dt
            step("preview of a real file of ~80 MB in under 5 s", real_file)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
