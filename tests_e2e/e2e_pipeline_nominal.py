"""The `nominal` profile in the Pipeline panel of mzFinder (TP_Mine/js/32_pipeline.js, 32_pipeline_form.js): a switch shown only after a unit-resolution file was previewed, its label and the
values it writes read from the step descriptor, a recipe text that carries `"preset":"nominal"` and leaves `floor` and `min_height` to the profile (tests/panel_recipe.py, the text the byte
test tests/test_feat_nominal_wasm.py compares with the CLI). Builds the site and the mzFinder .wasm like e2e_pipeline.py; needs cargo, wasm-bindgen and numpy (SKIP without).
Run from the repo root: python3 tests_e2e/e2e_pipeline_nominal.py"""
# covers: TP_Mine/
import json, os, shutil, socket, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); sys.path.insert(0, str(ROOT / "tests")); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from panel_recipe import PANEL_NOMINAL

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
from cargo_util import cli

tmp = Path(tempfile.mkdtemp(prefix="pipenominal_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", env)
else:                                                      # fresh sources of mzFinder over the site that was built
    for sub in ("js", "files"):
        for f in (ROOT / "TP_Mine" / sub).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
    ix = site / "static" / "mzfinder" / "indice.json"; d = json.loads(ix.read_text())
    d["js"] = sorted(f.name for f in (ROOT / "TP_Mine" / "js").glob("*.js")); ix.write_text(json.dumps(d))
data = site / "static" / "dati_pipeline"; data.mkdir(exist_ok=True)
# one high-resolution file and one unit-resolution file (both synthetic); the unit one is indexed by the same CLI the app uses
hr = tmp / "hr"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seed", "2", "--profile", "piccolo", "--scale", "1.0", "--out", str(hr)], cwd=ROOT, check=True, capture_output=True)
HR = sorted(hr.glob("*.mzidx"))[0]; shutil.copy2(HR, data / "hr_file.mzidx")
lr = tmp / "lr"
subprocess.run([sys.executable, "tools/lr_synthetic.py", str(lr), "--drift-series", "centroid", "--jitter", "0.3"], cwd=ROOT, check=True, capture_output=True)
mzml = sorted(lr.glob("drift-s*.mzML"))[:2]
listed = json.loads(cli("file", "index", "--cache", str(tmp / "cache"), *map(str, mzml)).stdout)["files"]
shutil.copy2(tmp / "cache" / listed[0]["key"], data / "lr_file.mzidx")
STEPS = json.loads((ROOT / "docs/schemi/steps-1.json").read_text(encoding="utf-8"))
NOM = next(s for s in STEPS if s["id"] == "features.detect")["presets"]["nominal"]

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
for _ in range(100):                                      # wait for the state (the port accepts), not for a fixed time
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: threading.Event().wait(0.1)
else:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True); raise SystemExit(f"the test server did not open port {PORT}")
URL = f"http://127.0.0.1:{PORT}/"
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_pipeline/" + n)).body.pipeTo(w); }
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.PIPE && window.UNTUI", timeout=120000)
        pg.evaluate(PUT, ["hr_file.mzidx", "lr_file.mzidx"])
        pg.evaluate("PIPE.open()"); pg.wait_for_selector("#pipe[open]")
        pg.wait_for_function("document.querySelectorAll('#pipe-file option').length === 2", timeout=60000)

        def preview(name):
            pg.select_option("#pipe-file", name); pg.click("#pipe-go")
            pg.wait_for_function("!PIPE.state.busy && PIPE.state.prev", timeout=120000)
            pg.wait_for_function("(n) => document.querySelector('#pipe-n').textContent.includes(n)", arg=name, timeout=120000)

        def hidden_before_a_preview():
            assert pg.locator("#pipe-nom").count() == 1 and pg.locator("#pipe-nom").is_hidden()
            assert "preset" not in json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))
        step("the switch is hidden before any file was previewed", hidden_before_a_preview)

        def hidden_on_hr():
            preview("hr_file.mzidx")
            assert pg.locator("#pipe-nom").is_hidden() and pg.evaluate("PIPE.state.prev.unit !== true"), pg.evaluate("PIPE.state.prev.unit")
        step("a high-resolution file does not offer the profile", hidden_on_hr)

        def shown_on_unit():
            preview("lr_file.mzidx")
            assert pg.evaluate("PIPE.state.prev.unit") is True and pg.locator("#pipe-nom").is_visible()
            assert not pg.is_checked("#pipe-nominal") and "preset" not in json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))
        step("a unit-resolution file offers the profile, off by default", shown_on_unit)

        def text_from_descriptor():
            label = pg.inner_text("#pipe-nom label").strip(); why = pg.inner_text("#pipe-nominal-why")
            assert label == NOM["title"], (label, NOM["title"])
            for k, v in NOM["params"].items(): assert f"{k} {v}" in why, (k, v, why)
            assert pg.get_attribute("#pipe-nominal", "aria-describedby") == "pipe-nominal-why"
        step("the label and the values it writes come from the step descriptor", text_from_descriptor)

        def recipe_on():
            before = json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))
            pg.focus("#pipe-nominal"); pg.keyboard.press("Space")                  # the keyboard works
            assert pg.is_checked("#pipe-nominal")
            r = json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))
            assert r == PANEL_NOMINAL, r                                            # the text the byte test sends to the .wasm, the app and the CLI
            assert "floor" not in r and "min_height" not in r and before.get("min_height") != NOM["params"]["min_height"]
            assert float(pg.input_value("#pipe-floor")) == NOM["params"]["floor"] and float(pg.input_value("#pipe-hmin")) == NOM["params"]["min_height"]
        step("on: the recipe carries the preset and leaves floor and min_height to it", recipe_on)

        def edited_height_wins():
            pg.fill("#pipe-hmin", "4000")
            r = json.loads(pg.evaluate("PIPE.recipe(PIPE.values())")); assert r["preset"] == "nominal" and r["min_height"] == 4000, r
            pg.fill("#pipe-hmin", str(NOM["params"]["min_height"]))
        step("a minimum height the user changes is written after the profile", edited_height_wins)

        def off_restores():
            pg.focus("#pipe-nominal"); pg.keyboard.press("Space")
            r = json.loads(pg.evaluate("PIPE.recipe(PIPE.values())"))
            assert "preset" not in r and r["min_height"] == 2e5 and pg.evaluate("PIPE.state.auto.floor") is True, r
            assert float(pg.input_value("#pipe-hmin")) == 2e5
        step("off: the fields go back to what they held", off_restores)

        def hr_file_clears_it():
            pg.check("#pipe-nominal"); preview("hr_file.mzidx")
            assert pg.locator("#pipe-nom").is_hidden() and not pg.is_checked("#pipe-nominal")
            assert "preset" not in json.loads(pg.evaluate("PIPE.recipe(PIPE.values())")) and float(pg.input_value("#pipe-hmin")) == 2e5
        step("previewing a high-resolution file switches the profile off and hides it", hr_file_clears_it)
    b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
