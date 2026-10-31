"""Flow «This browser» runs export.annotations@1 (flow/lazy/flow-web-export-annot.js, `flow_export_annot` of the `flowtab` .wasm): over the synthetic identity series (tools/feat_sintetici.py), a small MSP
library and a suspect list made here, the plan detect → align → gap filling → spectra.ms2 → spectra.match → formula.ms1 → screen.suspects → annot.level → export.annotations goes to `mzlab-cli flow run`, and the CSV
the browser keeps is the native file byte for byte (same SHA-256); every job is `done` with `via: "browser"`, a second run takes the CSV from OPFS (`reused`), no request is left unserved (`MZLAB_LR_MISS`),
and the semicolon variant of the same plan changes only the separator. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_export_annot.py"""
import hashlib, json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

try: import numpy  # noqa: F401
except ImportError: print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the .wasm"); sys.exit(0)

NAMES = ["SIN_blank", "SIN_dark", "SIN_t0", "SIN_t05min", "SIN_t10min", "SIN_t20min"]
def sample(n, i):
    kind, t = ("blank", None) if n.endswith("blank") else ("dark", None) if n.endswith("dark") else ("t0", 0) if n.endswith("t0") else ("sample", {"t05min": 5, "t10min": 10, "t20min": 20}[n.split("_")[1]])
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": i + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": kind, "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, i) for i, n in enumerate(NAMES)], "schema": "campioni/2"}, separators=(",", ":"))
MATCH = {"decoys": "spectrum", "top_n": 5, "tol_da": 0.02, "precursor_ppm": 10, "min_shared": 1, "min_score": 0.3}
PROTON = 1.00727646688

def mgf_blocks(text):
    out = []
    for b in text.split("BEGIN IONS")[1:]:
        head, peaks = {}, []
        for line in b.split("END IONS")[0].strip().splitlines():
            if "=" in line: k, v = line.split("=", 1); head[k] = v
            else:
                m, i = line.split()[:2]; peaks.append((float(m), float(i)))
        out.append((head, peaks))
    return out

IK = "RYYVLZVUVIJVGH-UHFFFAOYSA-N"
def msp(name, prec, peaks):
    return f"Name: {name}\nInChIKey: {IK}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.5f}\t{i:.1f}" for m, i in peaks) + "\n\n"


tmp = Path(tempfile.mkdtemp(prefix="flow_export_annot_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
if not (site / "static" / "wasm-lib" / "mzlab_lib_wasm.js").exists():
    ready = ROOT / "site" / "static" / "wasm-lib"
    if (ready / "mzlab_lib_wasm.js").exists(): shutil.copytree(ready, site / "static" / "wasm-lib")
    else:
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-lib-wasm"], cwd=ROOT, check=True, capture_output=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(site / "static" / "wasm-lib"), "target/wasm32-unknown-unknown/release/mzlab_lib_wasm.wasm"], cwd=ROOT, check=True)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_sintetici.py", str(ser), "--solo", "identita"], cwd=ROOT, check=True, capture_output=True)
run = tmp / "run"; run.mkdir()
for n in NAMES: shutil.copy2(ser / "identita" / (n + ".mzML"), run / (n + ".mzML"))
(run / "samples.json").write_text(SAMPLES)

def native(plan, name, *sets):
    (run / (name + ".json")).write_text(json.dumps(plan)); out = run / ("out_" + name)
    cmd = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / (name + ".json")), "--in", str(run), "--out", str(out)]
    for s_ in sets: cmd += ["--set", s_]
    subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True)
    return out

FILES = ["file:" + n + ".mzML" for n in NAMES]
def inputs(lib_sha, list_sha):
    return [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "lib.msp", "sha256": lib_sha, "type": "library"},
            {"name": "list.csv", "sha256": list_sha, "type": "library"}]
BASE = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": FILES}},
        {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
        {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
        {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
        {"id": "csv", "step": "export.table_csv@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
        {"id": "ms2", "step": "spectra.ms2@1", "in": {"source": FILES, "features": "csv.report"}}]
# 1. the MS2 of the features as MGF: the first spectrum is the library entry and the first suspect, the second one is another suspect
(run / "lib.msp").write_text(""); (run / "list.csv").write_text("name,inchikey,mass\n")
blocks = mgf_blocks((native({"format": "mzflow/2", "inputs": inputs("", ""), "steps": BASE}, "plan0") / "ms2.mgf").read_text())
assert len(blocks) >= 2, "the series has no MS2 spectrum for its features"
(h0, p0), (h1, _) = blocks[0], blocks[1]
LIB = msp("TruePair", float(h0["PEPMASS"].split()[0]), p0)
neutral = lambda h: float(h["PEPMASS"].split()[0]) - PROTON
frag = ";".join("%.4f" % m for m, _ in sorted(p0, key=lambda p: -p[1])[:3])
LIST = ("name,inchikey,mass,rt,fragments\n"
        f"Suspect one,AAAAAAAAAAAAAA,{neutral(h0):.5f},{float(h0['RTINSECONDS']) / 60:.3f},{frag}\n"
        f"Suspect two,BBBBBBBBBBBBBB,{neutral(h1):.5f},,\n")
(run / "lib.msp").write_text(LIB); (run / "list.csv").write_text(LIST)
LIBSHA, LISTSHA = hashlib.sha256(LIB.encode()).hexdigest(), hashlib.sha256(LIST.encode()).hexdigest()
# 2. the plan with the export
PLAN = {"format": "mzflow/2", "inputs": inputs(LIBSHA, LISTSHA), "steps": BASE + [
    {"id": "mat", "step": "spectra.match@1", "in": {"query": "ms2.spectra", "library": "file:lib.msp"}, "algorithm": "spectral_entropy", "params": MATCH},
    {"id": "f1", "step": "formula.ms1@1", "in": {"features": "csv.report", "index": FILES}, "params": {"tol_ppm": 5, "max_results": 5}},
    {"id": "sus", "step": "screen.suspects@1", "in": {"table": "gap.table", "suspects": ["file:list.csv"], "spectra": "ms2.spectra"}, "params": {"ppm": 10}},
    {"id": "lv", "step": "annot.level@1", "in": {"candidates": ["mat.hits", "sus.screen"], "formulas": "f1.formulas"}},
    {"id": "ex", "step": "export.annotations@1", "in": {"table": "gap.table", "candidates": ["mat.hits"], "screen": ["sus.screen"], "formulas": ["f1.formulas"], "annot": "lv.annot"}}]}
out = native(PLAN, "plan")
NAT = (out / "ex.csv").read_bytes()
lines = NAT.decode().splitlines()
assert lines[0].startswith("feature_id,mz,rt_min,level,level_why,method,rank,name,inchikey,formula,adduct,score,score_name,"), lines[0]
assert len(lines) > 3 and any(",library," in l for l in lines) and any(",suspect," in l for l in lines) and any(",formula," in l for l in lines), lines[:6]
SEMI = (native(PLAN, "plan_semi", "ex.separator=semicolon") / "ex.csv").read_text()
assert SEMI == NAT.decode().replace(",", ";"), "the semicolon variant changes more than the separator"
WEB = json.loads(json.dumps(PLAN))
# the page widens the one scan input of a plan to every file it holds
WEB["inputs"] = [i for i in WEB["inputs"] if i["type"] != "raw" or i["name"] == NAMES[0] + ".mzML"]
for s_ in WEB["steps"]:
    for k, v in s_["in"].items():
        if v == FILES: s_["in"][k] = [FILES[0]]
data = site / "static" / "dati_flow_export_annot"; data.mkdir()
for n in NAMES:
    shutil.copy2(out / ("idx-" + n + ".mzidx"), data / (n + ".mzidx")); shutil.copy2(run / (n + ".mzML"), data / (n + ".mzML"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_export_annot/" + n + ".mzidx")).body.pipeTo(w); }
}"""
HOLD = """async (names) => { for (const name of names) { const b = await (await fetch("static/dati_flow_export_annot/" + name + ".mzML")).blob(); window.MZLAB_RUST.noteUpload(name + ".mzML", b); } }"""
RUN = """async ([plan, samples, lib, list]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples; S.samplesJson['lib.msp'] = lib; S.samplesJson['list.csv'] = list;
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
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
sha = lambda b: hashlib.sha256(b if isinstance(b, bytes) else b.encode()).hexdigest()
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.on("request", lambda r: asked.append(r.url) if "/flowtab/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.wait_for_function("!!(window.MZLAB_RUST && window.MZLAB_RUST.noteUpload)", timeout=120000)
        pg.evaluate(HOLD, NAMES); pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        IDS = ["det", "ali", "gap", "ms2", "mat", "f1", "sus", "lv", "ex"]

        def first():
            r = pg.evaluate(RUN, [WEB, SAMPLES, LIB, LIST]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in IDS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, [e for e in r["ev"] if e["passo"] == i][-3:])
            assert asked, "the flowtab module was not fetched when export.annotations ran"
            web = pg.evaluate(KEPT).get("ex.csv"); assert web is not None
            at = next((i for i, (x, y) in enumerate(zip(NAT.decode(), web)) if x != y), min(len(NAT), len(web)))
            assert sha(web) == sha(NAT) and len(web) > 200, ("first difference at", at, NAT.decode()[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
        step("the browser gives the CSV of the native flow run, byte for byte", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [WEB, SAMPLES, LIB, LIST]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] == "ex" and e["stato"] == "running"], r["ev"]
            assert any(e["passo"] == "ex" and e["stato"] == "reused" and e["via"] == "browser" for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "ex"][-3:]
            assert len(asked) == before, "the second run fetched the module again"
        step("a second run takes the CSV from OPFS", again)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
