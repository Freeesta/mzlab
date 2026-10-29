"""Algorithm slot of `features.detect` in «This browser»: a plan with `algorithm: centwave` gives the browser the same feature table (SHA-256 of series.mzfeat) as the native run with the same layers
(`algorithm` written in the layer, as `flow run` writes it), and not the table of the default `masstrace`. Synthetic series only. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_detect_algorithm.py"""
import json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

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

tmp = Path(tempfile.mkdtemp(prefix="flow_web_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
data = site / "static" / "dati_flow_web"; data.mkdir()
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
for f in sorted(ser.glob("*.mzidx"))[:3]: shutil.copy2(f, data / f.name)
names = sorted(p.name for p in data.glob("*.mzidx"))

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):               # the server answers before the page is asked for
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n, { create: true })).createWritable(); await (await fetch("static/dati_flow_web/" + n)).body.pipeTo(w); }
}"""
# runs the plan of the open Flow page, resolves with the events it emitted and the error key if it failed
RUN = """async () => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  document.addEventListener('flow:web-step', on);
  let err = null;
  try { await FLOW.runners.web.run(S); } catch (e) { err = { key: e.key, params: e.params }; }
  document.removeEventListener('flow:web-step', on);
  return { ev, err };
}"""
CLEAR = """async () => { const m = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'); try { await m.removeEntry('flow', { recursive: true }); } catch (_) {} }"""
def of(r, passo, stato=None): return [e for e in r["ev"] if e["passo"] == passo and (stato is None or e["stato"] == stato)]
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, names)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        def native_sha(recipe):
            nat = tmp / "nat"; shutil.rmtree(nat, ignore_errors=True); nat.mkdir()
            # the detections carry the name of the file in the plan (`<stem>.mzML`), as `flow run` writes it
            named = tmp / "named"; named.mkdir(exist_ok=True)
            for n in names: shutil.copy2(data / n, named / (n.rsplit(".", 1)[0] + ".mzML"))
            out = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(nat), *[str(named / p.name) for p in sorted(named.iterdir())], "--recipe", recipe],
                                 cwd=ROOT, check=True, capture_output=True, text=True).stdout
            return dict(l.split(" ")[:2] for l in out.splitlines() if len(l.split(" ")) == 2 and len(l.split(" ")[1]) == 64).get("series.mzfeat")

        def browser_sha(algorithm):
            pg.evaluate(CLEAR)
            pg.evaluate("""(a) => { const inst = document.querySelector('.flx-list').__flow.plan.steps.find(s => String(s.step).split('@')[0] === 'features.detect');
              if (a) inst.algorithm = a; else delete inst.algorithm; }""", algorithm)
            r = pg.evaluate(RUN); assert r["err"] is None, r["err"]
            return pg.evaluate("""(async () => { const d = await navigator.storage.getDirectory(), f = await (await d.getDirectoryHandle('mzlab')).getDirectoryHandle('flow');
              const b = await (await (await f.getFileHandle('series.mzfeat')).getFile()).arrayBuffer();
              return [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join('') })()""")
        layers = lambda algorithm: json.dumps({"fill": True, "layers": [{"step": "features.detect", "params": {"algorithm": algorithm}}]} if algorithm else {"fill": True, "layers": []})
        state = {}

        def recipe_carries_algorithm():
            pg.evaluate("document.querySelector('.flx-list').__flow.plan.steps.find(s => String(s.step).split('@')[0] === 'features.detect').algorithm = 'centwave'")
            recipe = pg.evaluate("FLOW.runners.web.recipeOf(document.querySelector('.flx-list').__flow.plan)")
            assert '"algorithm":"centwave"' in recipe, recipe
        step("the recipe of the browser carries the algorithm of the detect step", recipe_carries_algorithm)

        def centwave_same_as_native():
            state["web_cw"] = browser_sha("centwave"); state["nat_cw"] = native_sha(layers("centwave"))
            assert state["web_cw"] == state["nat_cw"], state
        step("the browser table with algorithm=centwave has the SHA-256 of the native run", centwave_same_as_native)

        def default_same_as_native():
            web, nat = browser_sha(None), native_sha(layers(None))
            assert web == nat, (web, nat)
            assert web != state["nat_cw"], "centwave and masstrace give the same table on the series: the test proves nothing"
        step("the default algorithm still has the SHA-256 of the native run, and is not the centwave table", default_same_as_native)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
