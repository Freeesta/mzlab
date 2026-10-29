"""Flow «This browser» spectra.ms2, spectra.match (the algorithms other than tree) and table.match (flow/lazy/flow-web-ident.js, `flow_ms2` of the `hr` .wasm, `flow_match` of the `lib` .wasm,
`table_match_rows` of the `table` .wasm): over the synthetic identity series (tools/feat_sintetici.py) and a small MSP library made here (a true pair, a decoy, an entry that does not match) the browser
writes the MGF, the candidates of every algorithm (decoys on) and the candidates of the rows of the table that `mzlab-cli flow run` writes of the same plans, byte for byte; a second run takes them from OPFS.
Builds the site and the .wasm of mzFinder and of the library (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web_ident.py"""
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
ONE = "SIN_t10min"
def sample(n, i):
    kind, t = ("blank", None) if n.endswith("blank") else ("dark", None) if n.endswith("dark") else ("t0", 0) if n.endswith("t0") else ("sample", {"t05min": 5, "t10min": 10, "t20min": 20}[n.split("_")[1]])
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": i + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": kind, "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, i) for i, n in enumerate(NAMES)], "schema": "campioni/2"}, separators=(",", ":"))
ALGS = ["spectral_entropy", "cosine", "modified_cosine"]
MATCH = {"decoys": "spectrum", "top_n": 5, "tol_da": 0.02, "precursor_ppm": 10}
# min_score and min_shared belong to the entropy algorithm only
MATCH_OF = lambda a: {**MATCH, "min_shared": 1, "min_score": 0.3} if a == "spectral_entropy" else MATCH

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

tmp = Path(tempfile.mkdtemp(prefix="flow_web_ident_"))
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

def scans(files):
    return [{"name": n + ".mzML", "sha256": "", "type": "mzml"} for n in files]
# 1. the MS2 of one file as MGF, to make the library from
PLAN0 = {"format": "mzflow/2", "inputs": scans([ONE]), "steps": [{"id": "s", "step": "spectra.ms2@1", "in": {"source": ["file:" + ONE + ".mzML"]}}]}
mgf0 = (native(PLAN0, "plan0") / "s.mgf").read_text()
blocks = mgf_blocks(mgf0)
assert len(blocks) >= 2, "the series has no MS2 in " + ONE
# a true pair (the first spectrum), a decoy (the same precursor and peaks that do not belong to it) and an entry that does not match (another precursor)
(h0, p0), (h1, p1) = blocks[0], blocks[1]
mz0 = float(h0["PEPMASS"].split()[0])
LIB = msp("TruePair", mz0, p0) + msp("Decoy", mz0, [(m + 17.0, i) for m, i in reversed(p0)]) + msp("Elsewhere", mz0 + 120.0, p1)
(run / "lib.msp").write_text(LIB)
LIBSHA = hashlib.sha256(LIB.encode()).hexdigest()

# 2. the plans: spectra.ms2 → spectra.match (each algorithm, decoys on) on one file; the base chain → table.match (each algorithm) on the series
PLAN_A = {"format": "mzflow/2", "inputs": scans([ONE]) + [{"name": "lib.msp", "sha256": LIBSHA, "type": "library"}],
          "steps": [{"id": "s", "step": "spectra.ms2@1", "in": {"source": ["file:" + ONE + ".mzML"]}}]
          + [{"id": "m_" + a, "step": "spectra.match@1", "in": {"query": "s.spectra", "library": "file:lib.msp"}, "algorithm": a, "params": MATCH_OF(a)} for a in ALGS]}
PLAN_B = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "lib.msp", "sha256": LIBSHA, "type": "library"}],
          "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
                    {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                    {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                    {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]
          + [{"id": "t_" + a, "step": "table.match@1", "in": {"table": "gap.table", "samples": "file:samples.json", "index": ["idx.index"], "library": "file:lib.msp"}, "algorithm": a, "params": MATCH_OF(a)} for a in ALGS]}
out_a, out_b = native(PLAN_A, "plan_a"), native(PLAN_B, "plan_b")
NAT = {"s.mgf": (out_a / "s.mgf").read_bytes()}
for a in ALGS: NAT["m_" + a + ".json"] = (out_a / ("m_" + a + ".json")).read_bytes(); NAT["t_" + a + ".json"] = (out_b / ("t_" + a + ".json")).read_bytes()
print("native sizes:", {k: len(v) for k, v in NAT.items()})
data = site / "static" / "dati_flow_ident"; data.mkdir()
for n in NAMES: shutil.copy2(out_b / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))
shutil.copy2(run / (ONE + ".mzML"), data / (ONE + ".mzML"))

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
        pg.evaluate(HOLD, ONE)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def check(r, ids, expect):
            assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ids:
                assert any(e["passo"] == i and e["stato"] == expect and e["via"] == "browser" for e in r["ev"]), (i, expect, [e for e in r["ev"] if e["passo"] == i][-3:])
        def same(names):
            kept = pg.evaluate(KEPT)
            for name in names:
                assert name in kept, (name, list(kept))
                web, nat = kept[name].encode(), NAT[name]
                at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert sha(web) == sha(nat) and len(nat) > 40, (name, "first difference at", at, len(nat), len(web), nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])

        # the files of the series are in OPFS one at a time: the plan A reads the one open file, the plan B the six
        pg.evaluate(PUT, [ONE])
        IDS_A = ["s"] + ["m_" + a for a in ALGS]
        def first_a():
            r = pg.evaluate(RUN, [PLAN_A, SAMPLES, LIB]); check(r, IDS_A, "done")
            same(["s.mgf"] + ["m_" + a + ".json" for a in ALGS])
        step("spectra.ms2 and spectra.match (each algorithm, decoys on) write the files of the native flow run", first_a)
        def again_a():
            r = pg.evaluate(RUN, [PLAN_A, SAMPLES, LIB]); check(r, IDS_A, "reused")
        step("a second run of the plan takes them from OPFS", again_a)

        pg.evaluate(PUT, NAMES)
        IDS_B = ["t_" + a for a in ALGS]
        def first_b():
            r = pg.evaluate(RUN, [PLAN_B, SAMPLES, LIB]); check(r, IDS_B, "done")
            same(["t_" + a + ".json" for a in ALGS])
        step("table.match (each algorithm) writes the files of the native flow run", first_b)
        def again_b():
            r = pg.evaluate(RUN, [PLAN_B, SAMPLES, LIB]); check(r, IDS_B, "reused")
        step("a second run of the base chain and table.match takes them from OPFS", again_b)
        def no_miss():
            miss = pg.evaluate("(window.MZLAB_LR_MISS || []).length"); assert miss == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("the engine had no route it could not serve", no_miss)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
