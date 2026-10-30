"""Flow «This browser» runs qc.run_order@1 (the injection sequence of the sample table) and report.methods@1 (the methods text of a run record), both in the `flowtab` module: a synthetic series of 8 injections
in 2 batches, a sample table, a design of batches and the corsa/1 fixture of tests/golden/feat. The plan goes to `mzlab-cli flow run`; the texts the browser keeps are the native files byte for byte (same SHA-256); every job
is `done` with `via: "browser"`, a second run takes the texts from OPFS, no request is left unserved (`MZLAB_LR_MISS`), and the `flowtab` .wasm is fetched only when one of these steps runs.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_qcreport.py"""
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

# batch A: blank, two standards (1 and 10), QC, sample, QC; batch B: sample, QC (no blank, no standards: warnings)
ROWS = [("A", "blank", None), ("A", "std", 1.0), ("A", "std", 10.0), ("A", "qc", None), ("A", "sample", None), ("A", "qc", None), ("B", "sample", None), ("B", "qc", None)]
FILES = [{"name": f"RO_{k:02d}", "batch": b, "kind": kind, "conc": conc, "order": k + 1, "gain": 1.0} for k, (b, kind, conc) in enumerate(ROWS)]
NAMES = [f["name"] for f in FILES]
COMPOUNDS = [(301.1234, 1.0, 1.0e6), (357.2011, 1.6, 8.0e5)]   # m/z, RT (min), height
CYCLE, RT_FROM, RT_TO, SIGMA = 0.02, 0.6, 2.2, 0.04

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
        specs.append(sm._scan(n, rt, 1, mz[o], it[o], filt=sm.FILTRO1, res=60000)); tic.append(float(it.sum()))
    chroms = [ds._chrom(0, "TIC", rts, np.array(tic))]
    path.write_text(ds._hr_head("exploris", False, path.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")

def sample(f):
    return {"composto": "", "concentrazione": f["conc"], "file": f["name"] + ".mzML", "istd": False, "note": "", "ordine_iniezione": f["order"], "replica": 1, "serie": "", "sha256": "", "tempo": None, "tempo_confermato": False,
            "tempo_min": None, "tipo": f["kind"], "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(f) for f in FILES], "schema": "campioni/2"}, separators=(",", ":"))
DESIGN = json.dumps({"condizioni": [{"livelli": {}, "nome": "", "repliche": 1}], "confronti": [], "fattori": [], "lotti": [{"file": f["name"] + ".mzML", "lotto": f["batch"], "sha256": ""} for f in FILES],
                     "riferimento": None, "schema": "disegno/1"}, separators=(",", ":"))

RUN_RECORD = (ROOT / "tests" / "golden" / "feat" / "methods-corsa.json").read_text()

def plan(qc):
    """The plan: index of the files, and (with `qc`) the run-order check with and without the design, and the methods in both styles."""
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "mzml"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "design.json", "sha256": "", "type": "design"},
                                                                                       {"name": "run.json", "sha256": "", "type": "report"}]
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}}]
    if qc:
        st += [{"id": "ord", "step": "qc.run_order@1", "in": {"samples": "file:samples.json", "design": "file:design.json"}},
               {"id": "ord2", "step": "qc.run_order@1", "in": {"samples": "file:samples.json"}, "params": {"qc_every_max": 2, "blank_per_batch": False}},
               {"id": "met", "step": "report.methods@1", "in": {"run": "file:run.json"}},
               {"id": "met2", "step": "report.methods@1", "in": {"run": "file:run.json"}, "params": {"style": "list"}}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_qcreport_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for i, f in enumerate(FILES): write_mzml(run / (f["name"] + ".mzML"), f["gain"], 100 + i)
(run / "samples.json").write_text(SAMPLES); (run / "design.json").write_text(DESIGN); (run / "run.json").write_text(RUN_RECORD)
(run / "qcreport.json").write_text(json.dumps(plan(True)))
out = run / "out"
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "qcreport.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
data = site / "static" / "dati_flow_qcreport"; data.mkdir()
for n in NAMES: shutil.copy2(next(out.glob("*-" + n + ".mzidx")), data / (n + ".mzidx"))
OUTS = {"ord": "ord.json", "ord2": "ord2.json", "met": "met.md", "met2": "met2.md"}
NATIVE = {i: (out / f).read_bytes() for i, f in OUTS.items()}

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_qcreport/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan and the files of the plan in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, files]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = Object.assign(S.samplesJson || {}, files);
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

        FILES_OF_PLAN = {"samples.json": SAMPLES, "design.json": DESIGN, "run.json": RUN_RECORD}

        def upstream():
            r = pg.evaluate(RUN, [plan(False), FILES_OF_PLAN]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert any(e["passo"] == "idx" and e["stato"] in ("done", "reused") for e in r["ev"]), r["ev"][-6:]
            assert not asked, ("the flowtab module was fetched by a plan without qc.run_order and report.methods", asked)
        step("a plan without these steps does not fetch the flowtab module", upstream)

        def first():
            r = pg.evaluate(RUN, [plan(True), FILES_OF_PLAN]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in OUTS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert asked, "the flowtab module was not fetched when the steps ran"
            kept = pg.evaluate(KEPT)
            for i, f in OUTS.items():
                web = kept.get(f); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
                assert nat == web and len(web) > 50, (i, "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
            d = json.loads(kept["ord.json"]); assert d["schema"] == "runorder/1" and len(d["batches"]) == 2, kept["ord.json"][:300]
            assert [w["key"] for w in d["warnings"] if w["batch"] == "B"] == ["warn.qc.runorder.no_blank_batch", "warn.qc.runorder.std_levels"], d["warnings"]
            assert kept["met.md"].startswith("# Methods") and "features.detect" in kept["met.md"] and kept["met2.md"].count("\n- `") == 4, kept["met2.md"][:300]
        step("the browser gives the runorder/1 JSON and the methods Markdown of the native flow run, byte for byte", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(True), FILES_OF_PLAN]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in OUTS and e["stato"] == "running"], r["ev"]
            for i in OUTS: assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert len(asked) == before, "the second run fetched the module again"
        step("a second run takes the texts from OPFS", again)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
