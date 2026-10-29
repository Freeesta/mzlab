"""Flow «This browser» formula.ms1, formula.ms2 and annot.level (flow/lazy/flow-web-ident.js: `flow_ms1_rows` + `flow_ms1_join` of the `hr` .wasm, `flow_formula_ms2` and `flow_level` of the `attributio-all` .wasm):
over the synthetic identity series (tools/feat_sintetici.py) and a small MSP library made here (the first MS2 spectrum of the series is in it, the others are not) the browser writes the formulas of the features at
their MS1 apex, the sub-formulas that explain their MS2 and the level of each query as `mzlab-cli flow run` writes them for the same plan, byte for byte; a second run takes them from OPFS. One feature has a library
match (a level above the one without evidence), another has none. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web_ident2.py"""
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

def msp(name, prec, peaks):
    return f"Name: {name}\nPrecursorMZ: {prec}\nPrecursor_type: [M+H]+\nIon_mode: Positive\nNum Peaks: {len(peaks)}\n" + "\n".join(f"{m:.5f}\t{i:.1f}" for m, i in peaks) + "\n\n"

tmp = Path(tempfile.mkdtemp(prefix="flow_web_ident2_"))
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

def native(plan, name):
    (run / (name + ".json")).write_text(json.dumps(plan)); out = run / ("out_" + name)
    subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / (name + ".json")), "--in", str(run), "--out", str(out)], cwd=ROOT, check=True, capture_output=True)
    return out

FILES = ["file:" + n + ".mzML" for n in NAMES]
def inputs(lib_sha):
    return [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "lib.msp", "sha256": lib_sha, "type": "library"}]
BASE = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": FILES}},
        {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
        {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
        {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
        {"id": "csv", "step": "export.table_csv@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
        {"id": "ms2", "step": "spectra.ms2@1", "in": {"source": FILES, "features": "csv.report"}}]
# 1. the MS2 of the features as MGF, to make the library from: the first spectrum is in it with its own peaks
PLAN0 = {"format": "mzflow/2", "inputs": inputs(""), "steps": BASE}
(run / "lib.msp").write_text("")
blocks = mgf_blocks((native(PLAN0, "plan0") / "ms2.mgf").read_text())
assert len(blocks) >= 2, "the series has no MS2 spectrum for its features"
(h0, p0), (h1, p1) = blocks[0], blocks[1]
LIB = msp("TruePair", float(h0["PEPMASS"].split()[0]), p0)
LIBSHA = hashlib.sha256(LIB.encode()).hexdigest()
(run / "lib.msp").write_text(LIB)
# 2. the plan: the base chain, the MS2 of the features, the match against the library, the formulas and the level
PLAN = {"format": "mzflow/2", "inputs": inputs(LIBSHA), "steps": BASE + [
    {"id": "mat", "step": "spectra.match@1", "in": {"query": "ms2.spectra", "library": "file:lib.msp"}, "algorithm": "spectral_entropy", "params": MATCH},
    {"id": "f1", "step": "formula.ms1@1", "in": {"features": "csv.report", "index": FILES}, "params": {"tol_ppm": 5, "max_results": 5}},
    {"id": "f2", "step": "formula.ms2@1", "in": {"spectra": "ms2.spectra", "formulas": "f1.formulas"}},
    {"id": "lv", "step": "annot.level@1", "in": {"candidates": "mat.candidates", "formulas": "f1.formulas"}}]}
out = native(PLAN, "plan")
OUTS = ["f1.json", "f2.json", "lv.json"]
NAT = {n: (out / n).read_bytes() for n in OUTS}
print("native sizes:", {k: len(v) for k, v in NAT.items()})
levels = json.loads(NAT["lv.json"])["rows"]
assert any(r.get("name") == "TruePair" and r["level"] not in ("0", "5") for r in levels), levels
assert any("name" not in r for r in levels), "every feature has a match: the case without one is missing"
data = site / "static" / "dati_flow_ident"; data.mkdir()
for n in NAMES:
    shutil.copy2(out / ("idx-" + n + ".mzidx"), data / (n + ".mzidx")); shutil.copy2(run / (n + ".mzML"), data / (n + ".mzML"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_ident/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# the mzML the page has "open" for the steps that read it (the engine keeps the blob of every file it was given)
HOLD = """async (name) => { const b = await (await fetch("static/dati_flow_ident/" + name + ".mzML")).blob(); window.MZLAB_RUST.noteUpload(name + ".mzML", b); return b.size; }"""
RUN = """async ([plan, samples, lib]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples; S.samplesJson['lib.msp'] = lib;
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
errs = []
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
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        try: pg.wait_for_function("!!(window.MZLAB_RUST && window.MZLAB_RUST.noteUpload)", timeout=120000)
        except TimeoutError: raise TimeoutError("engine bridge missing: %s %s" % (pg.evaluate("window.MZLAB_RUST_ERR || ''"), errs[:3]))
        for n in NAMES: pg.evaluate(HOLD, n)
        pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def check(r, ids, expect):
            assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ids:
                assert any(e["passo"] == i and e["stato"] == expect and e["via"] == "browser" for e in r["ev"]), (i, expect, [e for e in r["ev"] if e["passo"] == i][-3:])
        IDS = ["f1", "f2", "lv"]
        def first():
            r = pg.evaluate(RUN, [PLAN, SAMPLES, LIB]); check(r, IDS, "done")
            kept = pg.evaluate(KEPT)
            for name in OUTS:
                assert name in kept, (name, list(kept))
                web, nat = kept[name].encode(), NAT[name]
                at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert sha(web) == sha(nat) and len(nat) > 40, (name, "first difference at", at, len(nat), len(web), nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
        step("formula.ms1, formula.ms2 and annot.level write the files of the native flow run", first)
        def again():
            r = pg.evaluate(RUN, [PLAN, SAMPLES, LIB]); check(r, IDS, "reused")
        step("a second run takes them from OPFS", again)
        def blocked():
            # an mzML that is not open in Data stays with the computer
            plan = json.loads(json.dumps(PLAN)); plan["steps"] = [s for s in plan["steps"] if s["id"] != "f1"] + [
                {"id": "g1", "step": "formula.ms1@1", "in": {"features": "csv.report", "index": ["file:SIN_elsewhere.mzML"]}}]
            plan["inputs"].append({"name": "SIN_elsewhere.mzML", "sha256": "", "type": "raw"})
            r = pg.evaluate(RUN, [plan, SAMPLES, LIB])
            assert any(e["passo"] == "g1" and e["stato"] == "blocked" for e in r["ev"]) or r.get("err"), [e for e in r["ev"] if e["passo"] == "g1"][-3:]
        step("a formula.ms1 over an mzML that is not open in Data is blocked for the computer", blocked)
        def no_miss():
            miss = pg.evaluate("(window.MZLAB_LR_MISS || []).length"); assert miss == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("the engine had no route it could not serve", no_miss)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
