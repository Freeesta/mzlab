"""Flow «This browser» runs quant.calibrate@1 (module `table`, `flow_quant`): a synthetic series of 8 injections with four compounds; the plan detect → align → gapfill → quant.calibrate (the signal as area, and as
height with a range and a dilution) goes to `mzlab-cli flow run`, and the CSV the browser keeps is the native file byte for byte (same SHA-256), the curve row without a peak is reported as a warning on the card;
every job is `done` with `via: "browser"`, a second run takes the texts from OPFS, no request is left unserved (`MZLAB_LR_MISS`). Synthetic data only.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_quant.py"""
# covers: flow/lazy/flow-web-quant.js, crates/mzlab-feat-table-wasm/src/quant.rs
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

# 2 batches of 17 injections, a QC at the positions 0, 4, 8, 12, 16 of each; the areas of batch A rise 2 % per injection, those of batch B fall 1.5 %
PER_BATCH = 4
FILES = []
for b in range(2):
    for k in range(PER_BATCH):
        FILES.append({"name": f"DR_{'AB'[b]}{k:02d}", "batch": "AB"[b], "kind": "qc" if k % 4 == 0 else "sample", "order": b * PER_BATCH + k + 1,
                      "gain": 1 + 0.02 * k if b == 0 else 1 - 0.015 * k})
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

def sample(f):
    return {"composto": "", "concentrazione": None, "file": f["name"] + ".mzML", "istd": False, "note": "", "ordine_iniezione": f["order"], "replica": 1, "serie": "", "sha256": "", "tempo": None, "tempo_confermato": False,
            "tempo_min": None, "tipo": f["kind"], "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(f) for f in FILES], "schema": "campioni/2"}, separators=(",", ":"))
CURVES = ("name,mz,rt,slope,intercept,unit,range_min,range_max,dilution\n"
          "alpha,301.1234,1.0,1000,0,ng/mL,,,1\n"
          "beta,357.2011,1.6,2000,500,ng/mL,200,900,2\n"
          "gamma,415.3102,2.2,1500,0,ng/mL,10,,1\n"
          "ghost,999.9,9.0,1,0,ng/mL,,,1\n")
IDS = ("qa", "qh")

def plan():
    """The plan: index → detect → align → gap filling, then the curves applied to the area and to the height."""
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "mzml"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "curves.csv", "sha256": "", "type": "report"}]
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
          {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
          {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
          {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]
    ins = {"table": "gap.table", "curves": "file:curves.csv", "samples": "file:samples.json"}
    st += [{"id": "qa", "step": "quant.calibrate@1", "in": ins, "params": {"kinds": "sample,qc"}},
           {"id": "qh", "step": "quant.calibrate@1", "in": ins, "params": {"signal": "height", "kinds": "sample"}}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_quant_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for i, f in enumerate(FILES): write_mzml(run / (f["name"] + ".mzML"), f["gain"], 100 + i)
(run / "samples.json").write_text(SAMPLES); (run / "curves.csv").write_text(CURVES)
(run / "quant.json").write_text(json.dumps(plan()))
out = run / "out"
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "quant.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
data = site / "static" / "dati_flow_quant"; data.mkdir()
for n in NAMES: shutil.copy2(next(out.glob("*-" + n + ".mzidx")), data / (n + ".mzidx"))
NATIVE = {i: (out / (i + ".csv")).read_bytes() for i in IDS}

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_quant/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan and the files of the plan in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, samples, curves]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples; S.samplesJson['curves.csv'] = curves;
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
        pg.on("request", lambda r: asked.append(r.url) if "/table/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        pg.evaluate(PUT, NAMES)

        def upstream():
            r = pg.evaluate(RUN, [plan(), SAMPLES, CURVES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("det", "ali", "gap"): assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])   # the gap filling is made with the alignment
        step("the upstream steps of the plan (detect, align, gap filling) run in the browser", upstream)

        def first():
            r = pg.evaluate(RUN, [plan(), SAMPLES, CURVES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in IDS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert asked, "the table module was not fetched when quant.calibrate ran"
            kept = pg.evaluate(KEPT)
            for i in IDS:
                web = kept.get(i + ".csv"); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
                assert nat == web and len(web) > 50, (i, "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
                assert web.startswith("compound,mz,rt,sample,signal,concentration,unit,flag\n") and ",ok\n" in web and "ghost" not in web, (i, web[:300])
        step("the browser gives the concentrations CSV of the native flow run, byte for byte (area, and height with a range and a dilution)", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(), SAMPLES, CURVES]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in IDS and e["stato"] == "running"], r["ev"]
            for i in IDS: assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert len(asked) == before, "the second run fetched the module again"
        step("a second run takes the texts from OPFS", again)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
