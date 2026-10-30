"""Flow «This browser» spectra.insilico with the downloadable candidate package (flow/lazy/flow-web-insilico.js, mzlab/web/insilico-scarica.js, docs/agenti/insilico.md):
`mzlab-cli insilico pack` writes a tiny invented package (several mass-range pieces) into the site; nothing of it is requested before the click on the button, the first click
shows the size (the catalogue only), the second stores the pieces in the Cache Storage `mzlab-libs`. The plan ms2 -> insilico then reaches `done` with `via: "browser"` and its
`candidates/1` is the file `mzlab-cli flow run` writes for the same plan, byte for byte; a run with the network cut after a reload still reads the pieces from the cache; the
engine had no route it could not serve; axe finds nothing serious on the button. Builds the site and the .wasm (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_insilico.py"""
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
AXE = lib.axe_path()
if not AXE: print("FAIL axe-core not available (pip install axe-playwright-python)"); sys.exit(1)

NAMES = ["SIN_blank", "SIN_dark", "SIN_t0", "SIN_t05min", "SIN_t10min", "SIN_t20min"]
def sample(n, i):
    kind, t = ("blank", None) if n.endswith("blank") else ("dark", None) if n.endswith("dark") else ("t0", 0) if n.endswith("t0") else ("sample", {"t05min": 5, "t10min": 10, "t20min": 20}[n.split("_")[1]])
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": i + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": kind, "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, i) for i, n in enumerate(NAMES)], "schema": "campioni/2"}, separators=(",", ":"))
PROTON = 1.00727646677

def precursors(mgf):
    return [float(l.split("=")[1].split()[0]) for l in mgf.splitlines() if l.startswith("PEPMASS=")]

tmp = Path(tempfile.mkdtemp(prefix="flow_insilico_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
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
def inputs(extra):
    return [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}] + extra
BASE = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": FILES}},
        {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
        {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
        {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
        {"id": "csv", "step": "export.table_csv@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
        {"id": "ms2", "step": "spectra.ms2@1", "in": {"source": FILES, "features": "csv.report"}}]
# 1. the MS2 of the features as MGF: the candidates are invented around their precursors (the mass column is explicit, the SMILES is any chain)
out0 = native({"format": "mzflow/2", "inputs": inputs([]), "steps": BASE}, "plan0")
prec = precursors((out0 / "ms2.mgf").read_text())
assert prec, "the series has no MS2 spectrum for its features"
rows = ["Identifier,CompoundName,SMILES,MolecularFormula,MonoisotopicMass"]
for i, m in enumerate(sorted(set(round(p - PROTON, 5) for p in prec))):
    rows.append(f"T{i},Target {i},CCCCCCCCO,C8H18O,{m}")
    rows.append(f"T{i}b,\"Near, {i}\",CCCCCCCCN,C8H19N,{m + m * 2e-6:.5f}")      # inside 5 ppm
for i in range(1500): rows.append(f"F{i:04d},Filler {i},CCCCCCCCCC,C10H22,{80 + i * 0.37:.5f}")      # other windows: other pieces
csv = tmp / "list.csv"; csv.write_text("\n".join(rows) + "\n")
pack = tmp / "pack"
cli = ROOT / "target" / "release" / "mzlab-cli"
subprocess.run(["cargo", "build", "--locked", "--release", "-q", "-p", "mzlab-cli"], cwd=ROOT, check=True)
subprocess.run([str(cli), "insilico", "pack", str(csv), str(pack), "--version", "test", "--doi", "10.0/test", "--piece-raw", "16000"], check=True, capture_output=True)
CAT = json.loads((pack / "catalogo.json").read_text())
assert len(CAT["pieces"]) >= 3, "the test package needs several pieces"
CATTEXT = (pack / "catalogo.json").read_text()
# 2. native: the catalogue is the suspects file, its pieces are next to it
for f in pack.iterdir(): shutil.copy2(f, run / f.name)
PLAN = {"format": "mzflow/2", "inputs": inputs([{"name": "catalogo.json", "sha256": hashlib.sha256(CATTEXT.encode()).hexdigest(), "type": "candidates"}]), "steps": BASE + [
    {"id": "ins", "step": "spectra.insilico@1", "in": {"spectra": "ms2.spectra", "suspects": "file:catalogo.json"}, "params": {"top": 5}}]}
out = native(PLAN, "plan")
NAT = (out / "ins.json").read_bytes()
hits = json.loads(NAT)["hits"]
assert hits and all(h["source"] == "insilico" for h in hits), "no hypothesis from the package"
WEB = json.loads(json.dumps(PLAN))
# the page widens the one scan input of a plan to every file it holds
WEB["inputs"] = [i for i in WEB["inputs"] if i["type"] != "raw" or i["name"] == NAMES[0] + ".mzML"]
for s_ in WEB["steps"]:
    for k, v in s_["in"].items():
        if v == FILES: s_["in"][k] = [FILES[0]]
# before the download the catalogue has no checksum: keeping the package changes the plan, which is what lets the scheduler offer the blocked step again
NOSHA = json.loads(json.dumps(WEB))
for i_ in NOSHA["inputs"]:
    if i_["name"] == "catalogo.json": i_["sha256"] = ""
print("native hits:", len(hits), "pieces:", len(CAT["pieces"]))
data = site / "static" / "dati_flow_ident"; data.mkdir()
for n in NAMES:
    shutil.copy2(out / ("idx-" + n + ".mzidx"), data / (n + ".mzidx")); shutil.copy2(run / (n + ".mzML"), data / (n + ".mzML"))
shutil.copytree(pack, site / "static" / "insilico")

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_ident/" + n + ".mzidx")).body.pipeTo(w); }
}"""
HOLD = """async (name) => { const b = await (await fetch("static/dati_flow_ident/" + name + ".mzML")).blob(); window.MZLAB_RUST.noteUpload(name + ".mzML", b); return b.size; }"""
# the plan on the page and the card of the candidates (what the results of the node draw), mounted in the document
MOUNT = """async ([plan, samples]) => {
  const S = document.querySelector('.flx-list').__flow;
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples; delete S.samplesJson['catalogo.json'];
  document.getElementById('insx')?.remove();
  const host = document.createElement('div'); host.id = 'insx'; document.body.append(host);
  for (const f of FLOW.extraResults) { const e = f(S, { ids: ['ins'] }); if (e) host.append(e); }
  return host.querySelectorAll('button').length;
}"""
RUN = """async (plan) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), out = {};
  for await (const [n, h] of d.entries()) if (h.kind === 'file') out[n] = await (await h.getFile()).text();
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
asked = {}
def count(route):
    k = route.request.url.split("/static/insilico/")[1]; asked[k] = asked.get(k, 0) + 1
    return route.continue_()
RUNNING = "ins"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", service_workers="block"); pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.route("**/static/insilico/**", count)
        def open_flow():
            pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_function("window.QTOOLS", timeout=120000)
            pg.wait_for_function("!!(window.MZLAB_RUST && window.MZLAB_RUST.noteUpload)", timeout=120000)
            for n in NAMES: pg.evaluate(HOLD, n)
            pg.evaluate(PUT, NAMES)
            pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
            pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && FLOW.webInsilico && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        open_flow()
        pg.evaluate(MOUNT, [NOSHA, SAMPLES])
        def silent():
            assert not asked, asked
        step("nothing of the package is requested before the click", silent)
        def blocked():
            r = pg.evaluate(RUN, NOSHA); assert not r.get("err"), r["err"]
            assert any(e["passo"] == "ins" and e["stato"] in ("blocked", "skipped") for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "ins"]
            assert not asked, asked
        step("without the package the step is blocked and tells so", blocked)
        def size():
            pg.get_by_role("button", name="Show the size of the in-silico candidate package").click()
            pg.get_by_role("button", name="Download the in-silico candidate package").wait_for(timeout=20000)
            assert list(asked) == ["catalogo.json"], asked
            assert str(max(1, round(CAT["size_compressed"] / 1048576))) + " MB" in pg.get_by_role("button", name="Download the in-silico candidate package").get_attribute("aria-label")
        step("the first click fetches the catalogue only and shows the size", size)
        def axe():
            if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=AXE)
            v = pg.evaluate("""() => axe.run(document.getElementById('insx'), {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
              .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.nodes.length]))""")
            assert not v, v
            box = pg.get_by_role("button", name="Download the in-silico candidate package").bounding_box(); assert box["width"] >= 24 and box["height"] >= 24, box
        step("axe finds nothing serious on the button and it is at least 24 px", axe)
        def install():
            pg.get_by_role("button", name="Download the in-silico candidate package").click()
            pg.wait_for_function("(n) => { const S = document.querySelector('.flx-list').__flow; return !!(S.samplesJson && S.samplesJson['catalogo.json']) }", timeout=60000)
            assert all(asked.get(x["file"]) == 1 for x in CAT["pieces"]), asked
            keys = pg.evaluate("async () => (await (await caches.open('mzlab-libs')).keys()).map(r => new URL(r.url).pathname).sort()")
            assert keys == sorted(["/static/insilico/catalogo.json"] + ["/static/insilico/" + x["file"] for x in CAT["pieces"]]), keys
        step("the second click stores the pieces in the Cache Storage", install)
        def first():
            r = pg.evaluate(RUN, WEB); assert not r.get("err"), (r["err"], r["ev"][-4:])
            for i in ("ms2", "ins"):
                assert any(e["passo"] == i and (e["stato"] == "done" or e.get("modo") == "reused") and e["via"] == "browser" for e in r["ev"]), (i, [e for e in r["ev"] if e["passo"] == i][-3:])
            kept = pg.evaluate(KEPT); assert "ins.json" in kept, list(kept)
            web = kept["ins.json"].encode(); at = next((i for i, (x, y) in enumerate(zip(NAT, web)) if x != y), min(len(NAT), len(web)))
            assert sha(web) == sha(NAT), ("first difference at", at, len(NAT), len(web), NAT[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
        step("ms2 -> insilico in this browser is the candidates/1 of mzlab-cli flow run, byte for byte", first)
        def again():
            r = pg.evaluate(RUN, WEB)
            assert any(e["passo"] == "ins" and e.get("modo") == "reused" for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "ins"]
        step("a second run takes it from OPFS", again)
        def offline():
            open_flow(); asked.clear()
            try:
                pg.evaluate(MOUNT, [WEB, SAMPLES])
                pg.get_by_role("button", name="Use the package stored in this browser").click(timeout=20000)
                pg.wait_for_function("() => { const S = document.querySelector('.flx-list').__flow; return !!(S.samplesJson && S.samplesJson['catalogo.json']) }", timeout=20000)
                def variant(top):
                    v = json.loads(json.dumps(WEB))
                    for s_ in v["steps"]:
                        if s_["id"] == "ins": s_["params"] = {"top": top}      # another key: the pieces are read again
                    return v
                r = pg.evaluate(RUN, variant(3)); assert not r.get("err"), r["err"]          # online: loads the .wasm of the step
                ctx.set_offline(True)
                r = pg.evaluate(RUN, variant(2)); assert not r.get("err"), r["err"]
                assert any(e["passo"] == "ins" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), [e for e in r["ev"] if e["passo"] == "ins"]
                g = json.loads(pg.evaluate(KEPT)["ins.json"])["hits"]
                assert 0 < len(g) < len(hits), (len(g), len(hits))      # top 2 of 5: fewer hypotheses than the native run
            finally: ctx.set_offline(False)
            assert not asked, asked
        step("after a reload with the network cut the cached package still runs", offline)
        def no_miss():
            miss = pg.evaluate("(window.MZLAB_LR_MISS || []).length"); assert miss == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("the engine had no route it could not serve", no_miss)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
