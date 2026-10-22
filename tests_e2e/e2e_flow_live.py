"""Flow «This browser», live (flow/lazy/flow-live.js, flow-live-jobs.js; scheduler of the engine api/flow/sched/<op>): opening Flow over three .mzidx in OPFS starts the detections without a click on Run;
changing a parameter of the alignment while they run does not redo them; changing one of the detection cancels only the detections that run; going back to the earlier value reuses what was kept.
It counts the jobs the pool receives (no fixed pauses) and measures `flow_live_primo_detect` (open Flow -> first detection running). Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and
numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_live.py"""
import json, os, time, shutil, subprocess, sys, tempfile, threading, urllib.request
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

tmp = Path(tempfile.mkdtemp(prefix="flow_live_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
data = site / "static" / "dati_flow_live"; data.mkdir()
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
  // written as .part and renamed: a half-written index is not an index (the live Flow lists the folder while Data writes)
  const parts = [];
  for (const n of names) { const h = await d.getFileHandle(n + ".part", { create: true }); await (await fetch("static/dati_flow_live/" + n)).body.pipeTo(await h.createWritable()); parts.push([h, n]); }
  for (const [h, n] of parts) await h.move(n);
}"""
# every event of the runner, and a clock for the first detection that runs
LISTEN = """() => { window.__ev = []; document.addEventListener('flow:web-step', e => window.__ev.push({ ...e.detail, t: performance.now() })); }"""
# what the pool receives: detections (held by a gate the test opens, cancellable like the real ones), alignments, cancellations
HOOK = """async () => {
  const p = await FLOW.runners.web.kit.getPool(), H = window.__h = { detect: [], align: 0, cancel: [], open: null, wait: new Map() };
  H.close = () => { H.gate = new Promise(r => { H.open = r; }); };
  const det = p.detectAll.bind(p), al = p.align.bind(p), can = p.cancelJob.bind(p);
  p.detectAll = async (files, recipe) => {
    const id = files[0].id; H.detect.push(id);
    const gate = H.gate;
    if (gate) {
      const dead = new Promise((_, no) => H.wait.set(id, () => no(Object.assign(new Error('cancelled'), { key: 'tpmine.feat.cancelled' }))));
      await Promise.race([gate, dead]);
    }
    return det(files, recipe);
  };
  p.align = (...a) => { H.align++; return al(...a); };
  p.cancelJob = id => { H.cancel.push(id); const w = H.wait.get(id); if (w) w(); return can(id); };
}"""
# the instance ids and the plan the scheduler holds
PLAN = """() => { const S = document.querySelector('.flx-list').__flow, f = n => S.plan.steps.find(s => String(s.step).split('@')[0] === n); return { detect: f('features.detect').id, align: f('features.align').id }; }"""
SET = """([id, name, value]) => { const S = document.querySelector('.flx-list').__flow, i = S.plan.steps.find(s => s.id === id); i.params = i.params || {}; i.params[name] = value; S.refresh(); }"""
NODES = """async () => (await (await fetch('api/flow/sched/nodes', { method: 'POST', body: '{}' })).json()).map(n => [n.instance, n.state])"""
def until(pg, fn, arg=None, timeout=60):
    """polls the function `fn` (a string) in the page until it gives a truthy value; wait_for_function of lib.py takes expressions only"""
    t0 = time.time()
    while not pg.evaluate(fn, arg):
        assert time.time() - t0 < timeout, fn[:160]
        lib.frames(pg, 6)
def settled(pg, text):
    """the scheduler has the plan with `text` in it (the page sends it 300 ms after the change)"""
    until(pg, "(t) => (FLOW.live.state().sent || '').includes(t)", text)
    pg.evaluate("FLOW.live.state().q")
def done(pg, ev_step, state):
    return pg.evaluate("([p, s]) => window.__ev.filter(e => e.passo === p && e.stato === s).length", [ev_step, state])
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def page():
            pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
            pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
            pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.QTOOLS", timeout=120000)
            return pg
        def open_flow(pg):
            pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000)
            pg.evaluate(LISTEN)
            pg.click('#nav button[data-v="flow"]')
            pg.wait_for_function("window.FLOW && FLOW.live && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        # --- A. the detections start by themselves, and the table is made, with no click on Run
        pg = page(); pg.evaluate(PUT, names)
        t0 = {}
        def by_itself():
            pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000)
            pg.evaluate(LISTEN); t0["t"] = pg.evaluate("performance.now()")
            pg.click('#nav button[data-v="flow"]')
            pg.wait_for_function("window.__ev.some(e => /detect/.test(e.passo) && e.stato === 'running')", timeout=120000)
            first = pg.evaluate("window.__ev.find(e => /detect/.test(e.passo) && e.stato === 'running').t")
            print("MISURA flow_live_primo_detect %.0f ms" % (first - t0["t"]))
            pg.wait_for_function("window.__ev.some(e => /align/.test(e.passo) && e.stato === 'done')", timeout=180000)
            ev = pg.evaluate("window.__ev"); assert not [e for e in ev if e["stato"] == "error" or e["stato"] == "refused"], ev
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        step("opening Flow over three .mzidx starts the detections and the alignment with no click on Run", by_itself)
        pg.context.close()

        # --- B. what a change redoes
        pg = page(); open_flow(pg)
        ids = pg.evaluate(PLAN); state = {}
        pg.evaluate(HOOK); pg.evaluate("window.__h.close()")
        pg.evaluate(PUT, names); pg.evaluate("FLOW.live.change()")
        pg.wait_for_function("window.__h.detect.length >= 1", timeout=120000)

        def align_change():
            n = pg.evaluate("window.__h.detect.length")
            pg.evaluate(SET, [ids["align"], "rt_gap", 0.2]); settled(pg, '"rt_gap":0.2')
            h = pg.evaluate("window.__h"); assert h["cancel"] == [] and len(h["detect"]) >= n and len(set(h["detect"])) == len(h["detect"]), h
        step("changing a parameter of the alignment while the detections run does not redo them", align_change)

        def finish():
            pg.evaluate("window.__h.open()")
            pg.wait_for_function("window.__ev.some(e => /align/.test(e.passo) && e.stato === 'done')", timeout=180000)
            h = pg.evaluate("window.__h"); assert sorted(h["detect"]) == sorted(set(h["detect"])) and len(h["detect"]) == len(names), h
            assert h["align"] == 1 and h["cancel"] == [], h
            state["detects"] = len(h["detect"])
        step("each file is detected once and the alignment runs once", finish)

        def detect_change():
            pg.evaluate("window.__h.close()")
            pg.evaluate(SET, [ids["detect"], "min_height", 150000]); settled(pg, '"min_height":150000')
            pg.wait_for_function("window.__h.detect.length > %d" % state["detects"], timeout=120000)
            pg.evaluate(SET, [ids["detect"], "min_height", 120000]); settled(pg, '"min_height":120000')
            pg.wait_for_function("window.__h.cancel.length >= 1", timeout=60000)
            h = pg.evaluate("window.__h")
            assert set(h["cancel"]) <= set(h["detect"][state["detects"]:]), h       # only detections that run, none of the finished ones
            assert all(c.startswith(ids["detect"]) for c in h["cancel"]), h          # no alignment is cancelled
            state["after"] = h
        step("changing a parameter of the detection cancels only the detections that run", detect_change)

        def back():
            pg.evaluate("(([id]) => { const S = document.querySelector('.flx-list').__flow, i = S.plan.steps.find(s => s.id === id); delete i.params.min_height; S.refresh(); })", [ids["detect"]])
            until(pg, "() => !(FLOW.live.state().sent || '').includes('min_height')")
            pg.evaluate("FLOW.live.state().q")
            n = pg.evaluate("window.__h.detect.length")      # the scheduler has the old plan again: from here the pool receives nothing
            until(pg, "async () => (await (await fetch('api/flow/sched/nodes', { method: 'POST', body: '{}' })).json()).every(n => n.state === 'done' || n.state === 'reused')")
            h = pg.evaluate("window.__h"); assert len(h["detect"]) == n and h["align"] == 1, (n, h)
            assert pg.evaluate("window.__ev.filter(e => /detect/.test(e.passo) && e.stato === 'reused').length") >= 1, pg.evaluate("window.__ev.slice(-6)")
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), pg.evaluate("window.MZLAB_LR_MISS")
        step("going back to the earlier value reuses what was kept: the pool receives nothing", back)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
