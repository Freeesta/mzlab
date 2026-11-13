"""Flow «This browser» runs table.plausible@1 (plausible/1 of the table, module `flowtab`): three synthetic files, the plan detect → align → gapfill → table.plausible (defaults and
`mz_max` 500) goes to `mzlab-cli flow run`, and the plausible/1 text the browser keeps is the native file byte for byte (same SHA-256); every job is `done` with `via: "browser"`, a second run takes
the texts from OPFS, no request is left unserved (`MZLAB_LR_MISS`), and the `flowtab` .wasm is fetched only when table.plausible runs.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_plausible.py"""
import hashlib, json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

try: import numpy as np
except ImportError: print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
import dati_sintetici as ds
from valida import sintetici_mzml as sm

FILES = [{"name": f"PL_{k:02d}", "gain": 1 + 0.05 * k} for k in range(3)]
NAMES = [f["name"] for f in FILES]
COMPOUNDS = [(301.1234, 1.0, 1.0e6), (357.2011, 1.6, 8.0e5), (415.3102, 2.2, 1.2e6), (523.4417, 2.8, 9.0e5)]   # m/z, RT (min), height
CYCLE, RT_FROM, RT_TO, SIGMA = 0.01, 0.6, 3.4, 0.04

def write_mzml(path, gain, seed):
    """One file: MS1 centroids every CYCLE min, the four compounds with their height times `gain`, and a little noise."""
    rng = np.random.default_rng(seed)
    rts = np.arange(RT_FROM, RT_TO, CYCLE)
    specs, tic = [], []
    for n, rt in enumerate(rts):
        mz = [rng.uniform(100, 900, 60)]; it = [rng.lognormal(np.log(8e3), 0.7, 60)]
        for m, r0, h in COMPOUNDS:
            y = h * gain * np.exp(-0.5 * ((rt - r0) / SIGMA) ** 2) * rng.normal(1, 0.01)
            if y >= 2e3: mz.append(np.array([m * (1 + rng.normal(0, 0.8e-6))])); it.append(np.array([y]))
        mz, it = np.concatenate(mz), np.concatenate(it); o = np.argsort(mz, kind="stable")
        specs.append(sm._scan(n, rt, 1, mz[o], it[o], filt=sm.FILTER1, res=60000)); tic.append(float(it.sum()))
    chroms = [ds._chrom(0, "TIC", rts, np.array(tic))]
    path.write_text(ds._hr_head("exploris", False, path.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")

def plan(check):
    """The plan: index → detect → align → gap filling, and (with `check`) the two plausibility checks."""
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "mzml"} for n in NAMES]
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
          {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
          {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
          {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]
    if check:
        st += [{"id": "pl", "step": "table.plausible@1", "in": {"table": "gap.table"}},
               {"id": "pl2", "step": "table.plausible@1", "in": {"table": "gap.table"}, "params": {"mz_max": 500}}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_plausible_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for i, f in enumerate(FILES): write_mzml(run / (f["name"] + ".mzML"), f["gain"], 100 + i)
(run / "plausible.json").write_text(json.dumps(plan(True)))
out = run / "out"
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plausible.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
data = site / "static" / "dati_flow_plausible"; data.mkdir()
for n in NAMES: shutil.copy2(next(out.glob("*-" + n + ".mzidx")), data / (n + ".mzidx"))
NATIVE = {i: (out / (i + ".json")).read_bytes() for i in ("pl", "pl2")}

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_plausible/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan and the files of the plan in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
# the texts the run kept in OPFS (mzlab/flow/msn/<key>.json = {name, text}), by name
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), msn = await d.getDirectoryHandle('msn'), out = {};
  for await (const [n, h] of msn.entries()) if (n !== 'index.json') { const j = JSON.parse(await (await h.getFile()).text()); out[j.name] = j.text; }
  return out;
}"""
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; asked = []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.on("request", lambda r: asked.append(r.url) if "/flowtab/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        pg.evaluate(PUT, NAMES)

        def upstream():
            r = pg.evaluate(RUN, [plan(False)]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("det", "ali", "gap"): assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])   # the gap filling is made with the alignment
            assert not asked, ("the flowtab module was fetched by a plan without table.plausible", asked)
        step("a plan without table.plausible does not fetch the flowtab module", upstream)

        def first():
            r = pg.evaluate(RUN, [plan(True)]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("pl", "pl2"): assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert asked, "the flowtab module was not fetched when table.plausible ran"
            kept = pg.evaluate(KEPT)
            for i in ("pl", "pl2"):
                web = kept.get(i + ".json"); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
                assert nat == web and len(web) > 50, (i, "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
                d = json.loads(web); assert d["schema"] == "plausible/1", (i, web[:200])
        step("the browser gives the plausible/1 of the native flow run, byte for byte (defaults and mz_max 500)", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(True)]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in ("pl", "pl2") and e["stato"] == "running"], r["ev"]
            for i in ("pl", "pl2"): assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert len(asked) == before, "the second run fetched the module again"
        step("a second run takes the texts from OPFS", again)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
