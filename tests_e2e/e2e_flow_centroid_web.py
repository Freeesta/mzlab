"""Flow «This browser» centroids low-resolution profile mzML itself (spectra.centroid -> rb.flowCentroid, the engine call of Settings; flow/lazy/flow-live-jobs.js): the «Feature table» plan on two profile P_FullMass files runs to the end,
no job is blocked by platform.computer, MZLAB_LR_MISS stays empty and series.mzfeat is the file of `mzlab-cli flow run`, byte for byte; a second run reuses the copy. Decoys: a high-resolution profile file still needs the desktop
app, a file of centroids is «already centroided» and makes no copy. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_centroid_web.py"""
import json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

try: import numpy as np
except ImportError: print("SKIP numpy is needed for the synthetic files"); sys.exit(0)
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
import dati_sintetici as ds

tmp = Path(tempfile.mkdtemp(prefix="flow_cen_web_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
rng = np.random.default_rng(5)
NAMES = ["P_FullMass-t0.mzML", "P_FullMass-t15.mzML"]
for n, t in zip(NAMES, (0, 15)): ds.full_scan_profile(run / n, t, rng)
hr = tmp / "hr"; hr.mkdir(); ds.hr_dda(hr / "HR_profile.mzML", np.random.default_rng(31), "exploris", profile=True)
cen = tmp / "cen"; cen.mkdir(); ds.full_scan(cen / "C_FullMass-t0.mzML", 0, np.random.default_rng(7))

# the Feature table plan, the files of the series as the source of its first step
BUILD = """(names) => {
  const S = document.querySelector('.flx-list').__flow, p = FLOW.exportPlan(FLOW.templatePlan(S.reg, FLOW.TEMPLATES.find(t => t.id === 'table')));
  const mz = p.inputs.find(i => i.type === 'mzml' || i.type === 'raw');
  p.steps.filter(s => s.in.source && [].concat(s.in.source)[0] === 'file:' + mz.name).forEach(s => { s.in.source = names.map(n => 'file:' + n); });
  p.inputs = names.map(n => ({ name: n, sha256: '', type: mz.type })).concat(p.inputs.filter(i => i !== mz));
  return p;
}"""
RUN = """async (plan) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
READ = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'); return Array.from(new Uint8Array(await (await (await d.getFileHandle('series.mzfeat')).getFile()).arrayBuffer())); }"""
CENS = """async () => { let d = await navigator.storage.getDirectory(); d = await (await d.getDirectoryHandle('mzlab')).getDirectoryHandle('idx'); const out = []; for await (const [n] of d.entries()) if (/\\.cen\\.(mzidx|key)$/.test(n)) out.push(n); return out.sort(); }"""
JOBS = """async () => { const S = document.querySelector('.flx-list').__flow, c = S.plan.steps.find(s => String(s.step).split('@')[0] === 'spectra.centroid');
  return c ? (await (await fetch('api/flow/sched/jobs', { method: 'POST', body: '{}' })).json()).filter(j => j.instance === c.id).map(j => [j.item, j.state, j.blocked_by || '']) : null; }"""
def open_flow(pg):
    pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
    pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
def fresh(b, files):
    pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
    pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
    pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
    pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.QTOOLS", timeout=120000)
    lib.stage(pg, files); lib.ready(pg); open_flow(pg)
    return pg

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = fresh(b, [run / n for n in NAMES]); plan = pg.evaluate(BUILD, NAMES)
        (run / "plan.json").write_text(json.dumps(plan)); out = run / "out"
        nat = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
        assert nat.returncode == 0, nat.stderr[-1500:]

        def first():
            r = pg.evaluate(RUN, plan); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert not [e for e in r["ev"] if "platform.computer" in json.dumps(e)], r["ev"]
            jobs = pg.evaluate(JOBS); assert jobs and all(j[1] in ("done", "cached", "reused") and not j[2] for j in jobs), jobs
            assert len(pg.evaluate(CENS)) == 4, pg.evaluate(CENS)      # a .cen.mzidx and a .cen.key for each of the two files
            web = bytes(pg.evaluate(READ)); table = sorted(out.glob("*.mzfeat"))
            assert len(table) == 1, list(out.iterdir())
            assert table[0].read_bytes() == web and len(web) > 100, (len(web), table[0].stat().st_size)
        step("two low-resolution profile files: the browser centroids them, no job waits for the desktop app, series.mzfeat is the file of flow run", first)

        def again():
            r = pg.evaluate(RUN, plan); assert not r.get("err"), r.get("err")
            assert len(pg.evaluate(CENS)) == 4, pg.evaluate(CENS)
        step("a second run takes the copies it made", again)
        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        pg.context.close()

        def decoy_hr():
            q = fresh(b, [hr / "HR_profile.mzML"]); pl = q.evaluate(BUILD, ["HR_profile.mzML"]); q.evaluate(RUN, pl)
            jobs = q.evaluate(JOBS); assert jobs and jobs[0][2] == "platform.computer", jobs
            assert q.evaluate(CENS) == [], q.evaluate(CENS)
            q.context.close()
        step("decoy: a high-resolution profile file still needs the desktop app and makes no copy", decoy_hr)

        def decoy_centroid():
            q = fresh(b, [cen / "C_FullMass-t0.mzML"]); pl = q.evaluate(BUILD, ["C_FullMass-t0.mzML"]); q.evaluate(RUN, pl)
            jobs = q.evaluate(JOBS); assert jobs and jobs[0][1] in ("cached", "done", "reused") and not jobs[0][2], jobs
            assert q.evaluate(CENS) == [], q.evaluate(CENS)
            q.context.close()
        step("decoy: a file of centroids is already centroided and makes no copy", decoy_centroid)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
