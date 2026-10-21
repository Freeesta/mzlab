"""Flow «This browser» runner (flow/lazy/flow-web-run.js, flow-web-ui.js, worker-flow-web.js): the base plan (index, detect, align, gapfill) over three synthetic .mzidx in OPFS emits
{passo, stato, mb, via, parte, di, modo} for every step, keeps each result in OPFS (mzlab/flow/), starts again from the first step not finished (a second run reuses everything), has the SHA-256 of the native run with the same recipe (layers), refuses a
series the budget does not hold with its numbers, and writes the note for phones and tablets. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web.py"""
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
        state = {}

        def first_run():
            r = pg.evaluate(RUN); state["first"] = r
            assert r["err"] is None, r["err"]
            det = of(r, "detect", "running"); assert [e["parte"] for e in det][-1] == len(names) and det[-1]["di"] == len(names), det
            assert of(r, "detect", "done") and all(e["via"] == "browser" for e in of(r, "detect")), r["ev"]
            al = of(r, "align", "running"); assert al and al[0]["mb"] and al[0]["modo"] in ("memory", "slabs"), al
            assert of(r, "align", "done"), r["ev"]
            assert of(r, "gapfill", "reused") and of(r, "index", "reused"), r["ev"]
            assert pg.evaluate("(async () => { const d = await navigator.storage.getDirectory(); const m = await d.getDirectoryHandle('mzlab'); const f = await m.getDirectoryHandle('flow'); return (await (await f.getFileHandle('series.mzfeat')).getFile()).size })()") > 0
        step("the base plan runs: events of every step, the table is kept in OPFS", first_run)

        def same_bytes_as_native():
            recipe = pg.evaluate("FLOW.runners.web.recipeOf(document.querySelector('.flx-list').__flow.plan)")
            assert '"layers"' in recipe, recipe
            got = pg.evaluate("""(async () => { const d = await navigator.storage.getDirectory(), f = await (await d.getDirectoryHandle('mzlab')).getDirectoryHandle('flow');
              const b = await (await (await f.getFileHandle('series.mzfeat')).getFile()).arrayBuffer();
              return [...new Uint8Array(await crypto.subtle.digest('SHA-256', b))].map(x => x.toString(16).padStart(2, '0')).join('') })()""")
            nat = tmp / "nat"; nat.mkdir()
            out = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "nativo", "--", str(nat), *[str(data / n) for n in names], "--recipe", recipe],
                                 cwd=ROOT, check=True, capture_output=True, text=True).stdout
            native = dict(l.split(" ")[:2] for l in out.splitlines() if len(l.split(" ")) == 2 and len(l.split(" ")[1]) == 64)
            assert native.get("series.mzfeat") == got, (native, got)
        step("the table of the browser has the SHA-256 of the native run with the same layers", same_bytes_as_native)

        def second_run():
            r = pg.evaluate(RUN); assert r["err"] is None, r["err"]
            assert all(e["modo"] == "reused" for e in of(r, "detect", "running") if e["parte"]), r["ev"]
            assert of(r, "align", "reused") and not of(r, "align", "running"), r["ev"]
        step("a second run reuses what is kept (nothing is computed again)", second_run)

        def refused():
            pg.evaluate(CLEAR)
            # a budget below what even the slabs need (the estimate counts whole megabytes)
            est = 0.001
            pg.evaluate("window.__rust = window.MZLAB_RUST; window.MZLAB_RUST = { budgetMb: %r }" % est)
            r = pg.evaluate(RUN)
            pg.evaluate("window.MZLAB_RUST = window.__rust")
            assert r["err"] and r["err"]["key"] == "error.featMemory", (est, r["err"], r["ev"][-3:])
            assert of(r, "align", "refused") and r["err"]["params"]["stima_mb"] > est, r
        step("a series the budget does not hold is refused with its numbers", refused)

        def phone_note():
            pg.evaluate("window.MZLAB_PHONE = true")
            pg.click("#flx-run"); pg.wait_for_selector("#flx-web-why")
            assert "made for computers" in pg.inner_text("#flx-web-why"), pg.inner_text("#flx-web-why")
            btn = pg.locator('.fl-meta button', has_text="Run in this browser"); assert btn.count() == 1
            box = btn.bounding_box(); assert box["height"] >= 24 and box["width"] >= 24, box
            pg.evaluate("window.MZLAB_PHONE = false")
        step("a phone or tablet reads the note for computers; the Run button is a reachable target", phone_note)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
