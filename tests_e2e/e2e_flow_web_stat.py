"""Flow «This browser» runs table.stats@1, table.pca@1 and table.kmd@1 (module `flowstat`): a synthetic series of 2 blanks and 2 conditions of 4 injections (some compounds up in the treatment,
a CH2 homologous series for the Kendrick mass defect). The plan detect → align → gapfill → stats, pca (two scalings) and kmd (CF2 and CH2) goes to `mzlab-cli flow run`, and every text the browser
keeps is the native file byte for byte (same SHA-256); every job is `done` with `via: "browser"`, a second run takes the texts from OPFS, no request is left unserved (`MZLAB_LR_MISS`), and the
`flowstat` .wasm is fetched only when one of these steps runs. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web_stat.py"""
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

# 2 blanks, then 4 references and 4 treatments; `gain` multiplies each compound (the blank has only the last one)
FILES = [{"name": f"ST_B{k}", "kind": "blank", "series": "", "gains": [0, 0, 0, 0, 0, 0, 1]} for k in range(2)]
FILES += [{"name": f"ST_R{k}", "kind": "sample", "series": "ref", "gains": [1, 1, 1, 1, 1, 1, 1]} for k in range(4)]
FILES += [{"name": f"ST_T{k}", "kind": "sample", "series": "trt", "gains": [4, 1, 0.25, 1.1, 1.3, 1.6, 1]} for k in range(4)]
for i, f in enumerate(FILES): f["order"] = i + 1
NAMES = [f["name"] for f in FILES]
# m/z, RT (min), height: three up/down/equal, a CH2 homologous series of three, and a compound of the blank
COMPOUNDS = [(301.1234, 1.0, 1.0e6), (357.2011, 1.6, 8.0e5), (415.3102, 2.2, 1.2e6), (283.2632, 1.3, 7.0e5), (297.2788, 1.9, 7.5e5), (311.2945, 2.5, 8.0e5), (523.4417, 2.8, 9.0e5)]
CYCLE, RT_FROM, RT_TO, SIGMA = 0.01, 0.6, 3.4, 0.04

def write_mzml(path, gains, seed):
    """One file: MS1 centroids every CYCLE min, the compounds with their height times their `gains`, and a little noise."""
    rng = np.random.default_rng(seed)
    rts = np.arange(RT_FROM, RT_TO, CYCLE)
    specs, tic = [], []
    for n, rt in enumerate(rts):
        mz = [rng.uniform(100, 900, 60)]; it = [rng.lognormal(np.log(8e3), 0.7, 60)]
        for (m, r0, h), gain in zip(COMPOUNDS, gains):
            y = h * gain * np.exp(-0.5 * ((rt - r0) / SIGMA) ** 2) * rng.normal(1, 0.01)
            if y >= 2e3: mz.append(np.array([m * (1 + rng.normal(0, 0.8e-6))])); it.append(np.array([y]))
        mz, it = np.concatenate(mz), np.concatenate(it); o = np.argsort(mz, kind="stable")
        specs.append(sm._scan(n, rt, 1, mz[o], it[o], filt=sm.FILTRO1, res=60000)); tic.append(float(it.sum()))
    chroms = [ds._chrom(0, "TIC", rts, np.array(tic))]
    path.write_text(ds._hr_head("exploris", False, path.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")

def sample(f):
    return {"composto": "", "concentrazione": None, "file": f["name"] + ".mzML", "istd": False, "note": "", "ordine_iniezione": f["order"], "replica": 1, "serie": f["series"], "sha256": "", "tempo": None,
            "tempo_confermato": False, "tempo_min": None, "tipo": f["kind"], "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(f) for f in FILES], "schema": "campioni/2"}, separators=(",", ":"))
DESIGN = json.dumps({"condizioni": [{"livelli": {}, "nome": n, "repliche": 4} for n in ("ref", "trt")], "confronti": [{"tipo": "vs_reference"}], "fattori": [], "lotti": [],
                     "riferimento": "ref", "schema": "disegno/1"}, separators=(",", ":"))
IDS = ("st", "pc", "pa", "kf", "kc")

def plan(stat):
    """The plan: index → detect → align → gap filling, and (with `stat`) the statistics of the table."""
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "mzml"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "design.json", "sha256": "", "type": "design"}]
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
          {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
          {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
          {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]
    if stat:
        st += [{"id": "st", "step": "table.stats@1", "in": {"table": "gap.table", "samples": "file:samples.json", "design": "file:design.json"}},
               {"id": "pc", "step": "table.pca@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
               {"id": "pa", "step": "table.pca@1", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": {"scaling": "auto", "ncomp": 3}},
               {"id": "kf", "step": "table.kmd@1", "in": {"table": "gap.table"}},
               {"id": "kc", "step": "table.kmd@1", "in": {"table": "gap.table"}, "params": {"unit": "CH2", "tol_mda": 2}}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_stat_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for i, f in enumerate(FILES): write_mzml(run / (f["name"] + ".mzML"), f["gains"], 100 + i)
(run / "samples.json").write_text(SAMPLES); (run / "design.json").write_text(DESIGN)
(run / "stat.json").write_text(json.dumps(plan(True)))
out = run / "out"
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "stat.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
data = site / "static" / "dati_flow_stat"; data.mkdir()
for n in NAMES: shutil.copy2(next(out.glob("*-" + n + ".mzidx")), data / (n + ".mzidx"))
NATIVE = {i: (out / (i + ".json")).read_bytes() for i in IDS}

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_stat/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan and the files of the plan in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, samples, design]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples; S.samplesJson['design.json'] = design;
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
# the scheduler's jobs of one instance: [item, state, blocked_by]
JOBS = """async (id) => (await (await fetch('api/flow/sched/jobs', { method: 'POST', body: '{}' })).json()).filter(j => j.instance === id).map(j => [j.item, j.state, j.blocked_by || ''])"""
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
        pg.on("request", lambda r: asked.append(r.url) if "/flowstat/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        pg.evaluate(PUT, NAMES)

        def upstream():
            r = pg.evaluate(RUN, [plan(False), SAMPLES, DESIGN]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("det", "ali", "gap"): assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])   # the gap filling is made with the alignment
            assert not asked, ("the flowstat module was fetched by a plan without the statistics", asked)
        step("a plan without the statistics does not fetch the flowstat module", upstream)

        def first():
            r = pg.evaluate(RUN, [plan(True), SAMPLES, DESIGN]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in IDS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert asked, "the flowstat module was not fetched when the statistics ran"
            kept = pg.evaluate(KEPT)
            for i in IDS:
                web = kept.get(i + ".json"); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
                assert nat == web and len(web) > 50, (i, "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
            assert json.loads(kept["st.json"])["schema"] == "stat/1", kept["st.json"][:200]
            assert kept["pc.json"] != kept["pa.json"] and kept["kf.json"] != kept["kc.json"], "the parameters did not reach the engine"
            assert json.loads(kept["kc.json"]).get("series"), ("the CH2 series was not found", kept["kc.json"][:400])
        step("the browser gives the stat/1, PCA and Kendrick texts of the native flow run, byte for byte", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(True), SAMPLES, DESIGN]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in IDS and e["stato"] == "running"], r["ev"]
            for i in IDS: assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert len(asked) == before, "the second run fetched the module again"
        step("a second run takes the texts from OPFS", again)

        def blocked():
            # none.json is an input of the plan, but the page holds no text for it: the executor leaves the step to the computer
            p2 = plan(True); p2["inputs"].append({"name": "none.json", "sha256": "", "type": "design"})
            p2["steps"] = [s for s in p2["steps"] if s["id"] != "st"] + [{"id": "sx", "step": "table.stats@1", "in": {"table": "gap.table", "samples": "file:samples.json", "design": "file:none.json"}}]
            r = pg.evaluate(RUN, [p2, SAMPLES, DESIGN])
            assert not any(e["passo"] == "sx" and e["stato"] in ("done", "running") for e in r["ev"]), r["ev"][-6:]
            jobs = pg.evaluate(JOBS, "sx"); assert jobs and all(j[1] == "blocked" and j[2] == "platform.computer" for j in jobs), jobs
        step("table.stats without its design stays at the computer", blocked)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
