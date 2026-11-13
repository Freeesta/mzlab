"""Flow «This browser» runs structure.render@1 (.wasm `interp`), structure.reactivity@1 (.wasm `chem`) and msn.figure@1 (.wasm `msnfigura`): over the synthetic identity series the tp chain of
e2e_flow_tp_post.py gives the `sites/1` of tp.position, which structure.render draws over the parent; structure.reactivity reads a parent file (paracetamol), msn.figure the public synthetic graph
`tests_e2e/fixtures/msngrafo_esempio.json` (an SVG in tree view and an HTML in radial view). Every text the browser keeps is the file of `mzlab-cli flow run` of the same plan, byte for byte; a second run
takes the texts from OPFS; the `chem` and `msnfigura` modules are fetched only when their steps run; a PNG figure stays with the computer. Builds the site and the .wasm of mzFinder (needs cargo,
wasm-bindgen, python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_web_struct.py"""
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
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)

NAMES = ["SIN_blank", "SIN_dark", "SIN_t0", "SIN_t05min", "SIN_t10min", "SIN_t20min"]
PARENT = json.dumps({"formula": "C22H31N3O4S", "rt": 1.0})
PARENT2 = json.dumps({"smiles": "CCN(CC)C(=O)c1ccc(cc1)S(=O)(=O)N", "rt": 1.0})
PARENT3 = "CC(=O)Nc1ccc(O)cc1"
GRAPH = (HERE / "fixtures" / "msngrafo_esempio.json").read_text(encoding="utf-8")
def sample(n, i):
    kind, t = ("blank", None) if n.endswith("blank") else ("dark", None) if n.endswith("dark") else ("t0", 0) if n.endswith("t0") else ("sample", {"t05min": 5, "t10min": 10, "t20min": 20}[n.split("_")[1]])
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": i + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": kind, "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, i) for i, n in enumerate(NAMES)], "schema": "campioni/2"}, separators=(",", ":"))
# instance id -> the name of its file in the native output folder
OUTS = {"drw": "drw.json", "rx": "rx.json", "rn": "rn.json", "fs": "fs.svg", "fh": "fh.html"}

def plan(struct, png=False):
    """The tp chain up to tp.position, and (with `struct`) the three steps of this test; `png` adds a figure the browser leaves to the computer."""
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "parent.json", "sha256": "", "type": "structure"},
              {"name": "parent2.json", "sha256": "", "type": "structure"}, {"name": "parent3.txt", "sha256": "", "type": "structure"}, {"name": "graph.json", "sha256": "", "type": "report"}]
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
          {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
          {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
          {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
          {"id": "gen", "step": "tp.generate@1", "in": {"parent": "file:parent.json"}},
          {"id": "lnk", "step": "tp.link@2", "in": {"table": "gap.table", "samples": "file:samples.json", "parent": "file:parent.json", "index": ["idx.index"], "hypotheses": "gen.hypotheses"}},
          {"id": "pos", "step": "tp.position@1", "in": {"tp": "lnk.tp", "parent": "file:parent2.json"}}]
    if struct:
        st += [{"id": "drw", "step": "structure.render@1", "in": {"sites": "pos.sites", "parent": "file:parent2.json"}},
               {"id": "rx", "step": "structure.reactivity@1", "in": {"parent": "file:parent3.txt"}, "params": {"conformers": 1}},
               {"id": "rn", "step": "structure.reactivity@1", "in": {"parent": "file:parent3.txt"}, "params": {"conformers": 1, "solvent": "none"}},
               {"id": "fs", "step": "msn.figure@1", "in": {"graph": "file:graph.json"}},
               {"id": "fh", "step": "msn.figure@1", "in": {"graph": "file:graph.json"}, "params": {"format": "html", "view": "radial"}}]
    if png:
        st += [{"id": "fp", "step": "msn.figure@1", "in": {"graph": "file:graph.json"}, "params": {"format": "png"}}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_struct_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_synthetic.py", str(ser), "--solo", "identita"], cwd=ROOT, check=True, capture_output=True)
run = tmp / "run"; run.mkdir()
for n in NAMES: shutil.copy2(ser / "identita" / (n + ".mzML"), run / (n + ".mzML"))
for name, text in (("samples.json", SAMPLES), ("parent.json", PARENT), ("parent2.json", PARENT2), ("parent3.txt", PARENT3), ("graph.json", GRAPH)): (run / name).write_text(text, encoding="utf-8")
(run / "plan.json").write_text(json.dumps(plan(True)))
nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, capture_output=True, text=True)
assert nat_run.returncode == 0, nat_run.stderr[-1500:]
NATIVE = {i: (run / "out" / n).read_bytes() for i, n in OUTS.items()}
data = site / "static" / "dati_flow_struct"; data.mkdir()
for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_struct/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan and its files in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, files]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = files['samples.json']; S.samplesJson['graph.json'] = files['graph.json'];
  for (const n of ['parent.json', 'parent2.json', 'parent3.txt']) await FLOW.parentSet(S, S.plan.inputs.find(i => i.name === n), files[n]);
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
# why the scheduler holds the jobs of one instance: the blocked_by of its events (the jobs and nodes routes do not carry it)
JOBS = """async (id) => { const r = await (await fetch('api/flow/sched/events', { method: 'POST', body: '{"since":0}' })).json(), all = Array.isArray(r) ? r : (r.events || []);
  return all.filter(e => e.instance === id && e.state === 'blocked').map(e => [e.item, e.state, e.blocked_by || '']); }"""
FILES = {"samples.json": SAMPLES, "parent.json": PARENT, "parent2.json": PARENT2, "parent3.txt": PARENT3, "graph.json": GRAPH}
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
        pg.on("request", lambda r: asked.append(r.url) if "/msnfigura/" in r.url or "/chem/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def upstream():
            r = pg.evaluate(RUN, [plan(False), FILES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert any(e["passo"] == "pos" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), r["ev"][-6:]
            assert not asked, ("the chem or msnfigura module was fetched by a plan without their steps", asked)
        step("a plan without these steps does not fetch the chem and msnfigura modules", upstream)

        def first():
            r = pg.evaluate(RUN, [plan(True), FILES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in OUTS: assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert any("/chem/" in u for u in asked) and any("/msnfigura/" in u for u in asked), asked
            kept = pg.evaluate(KEPT)
            for i, n in OUTS.items():
                web = kept.get(n); assert web is not None, (i, list(kept))
                nat = NATIVE[i].decode()
                at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert nat == web and len(web) > 50, (i, len(nat), len(web), "first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
                assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE[i]).hexdigest(), i
            d = json.loads(kept["drw.json"]); assert d["schema"] == "drawings/1" and d["rows"], kept["drw.json"][:300]
            assert kept["rx.json"] != kept["rn.json"], "the solvent did not reach the engine"
            assert kept["fs.svg"].startswith("<svg") and 'data-mode="tree"' in kept["fs.svg"], kept["fs.svg"][:200]
            assert kept["fh.html"].lstrip().lower().startswith("<!doctype html"), kept["fh.html"][:200]
        step("the browser gives drawings/1, reatt/1, the SVG and the HTML of the native flow run, byte for byte", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(True), FILES]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in OUTS and e["stato"] == "running"], r["ev"]
            for i in OUTS: assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert len(asked) == before, ("the second run fetched a module again", asked[before:])
        step("a second run takes the texts from OPFS", again)

        def png():
            r = pg.evaluate(RUN, [plan(True, png=True), FILES])
            assert not any(e["passo"] == "fp" and e["stato"] in ("done", "running") for e in r["ev"]), r["ev"][-6:]
            jobs = pg.evaluate(JOBS, "fp"); assert jobs and all(j[1] == "blocked" and j[2] == "platform.computer" for j in jobs), jobs
        step("a PNG figure stays at the computer", png)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
