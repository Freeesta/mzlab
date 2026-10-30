"""Flow «This browser» qc.mass_accuracy (flow-web-qc.js, `mass_accuracy` of the `hr` .wasm): over the five demo full-scan files the browser writes the massacc/1 report (before the alignment, from the indexes alone, and after it, with the
offset model correcting the feature table) and the corrected .mzfeat that `mzlab-cli flow run` writes of the same plan, byte for byte; a second run takes them from OPFS; the view draws the table and the two charts without axe violations.
Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and python3: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_qc_mass.py"""

import hashlib, json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)

# the demo files under names that sort like the plan: the browser lists the indexes of OPFS by name, `flow run` takes the order of the plan
NAMES = ["FullScan_t00", "FullScan_t05", "FullScan_t10", "FullScan_t15", "FullScan_t30"]
TIMES = [0, 5, 10, 15, 30]
def sample(n, t):
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": NAMES.index(n) + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": "t0" if t == 0 else "sample", "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, t) for n, t in zip(NAMES, TIMES)], "schema": "campioni/2"}, separators=(",", ":"))
REFS = "name,ion,role\na,141.18,reference\nb,158.20,reference\nc,149.21,reference\nd,198.35,reference\ne,214.25,reference\n"
P = {"min_refs": 1, "min_intensity": 1000}
PLAN = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "refs.csv", "sha256": "", "type": "report"}],
        "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
                  {"id": "qa", "step": "qc.mass_accuracy@1", "in": {"raw": ["idx.index"], "samples": "file:samples.json", "references": "file:refs.csv"}, "params": P},
                  {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                  {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                  {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
                  {"id": "qb", "step": "qc.mass_accuracy@1", "in": {"raw": ["idx.index"], "table": "gap.table", "samples": "file:samples.json", "references": "file:refs.csv"}, "params": dict(P, model="offset")}]}

tmp = Path(tempfile.mkdtemp(prefix="flow_qc_mass_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in zip(NAMES, TIMES): shutil.copy2(ROOT / "mzlab" / "web" / "esempi" / ("FullScan_t%d.mzML" % t), run / (n + ".mzML"))
(run / "plan.json").write_text(json.dumps(PLAN)); (run / "samples.json").write_text(SAMPLES); (run / "refs.csv").write_text(REFS)
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, check=True, capture_output=True)
print("native outputs:", sorted(x.name for x in (run / "out").iterdir() if not x.name.endswith(".mzidx")))
data = site / "static" / "dati_flow_qc"; data.mkdir()
for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_qc/" + n + ".mzidx")).body.pipeTo(w); }
}"""
RUN = """async ([plan, samples, refs]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples; S.samplesJson['refs.csv'] = refs;
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
TABLE = """async () => {
  let d = await navigator.storage.getDirectory();
  for (const p of ['mzlab', 'flow', 'live']) d = await d.getDirectoryHandle(p);
  const bytes = [];
  for await (const [n, h] of d.entries()) if (n.startsWith('massacc-') && n.endsWith('.mzfeat')) bytes.push(new Uint8Array(await (await h.getFile()).arrayBuffer()));
  if (bytes.length !== 1) return 'files:' + bytes.length;
  const h = await crypto.subtle.digest('SHA-256', bytes[0]);
  return [...new Uint8Array(h)].map(x => x.toString(16).padStart(2, '0')).join('');
}"""
VIEW = """async () => {
  const name = 'qb.json';
  document.getElementById('qcview')?.remove();
  const host = document.createElement('div'); host.id = 'qcview'; host.className = 'flx'; document.body.append(host);
  const kept = {}; const msn = await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow')).getDirectoryHandle('msn');
  for await (const [n, h] of msn.entries()) if (n !== 'index.json') { const j = JSON.parse(await (await h.getFile()).text()); kept[j.name] = j.text; }
  await FLOW.VIEWS.find(v => v.id === 'massacc').open(host, { label: name, text: async () => kept[name] });
}"""
AXE = lib.axe_path()
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []
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
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def first():
            r = pg.evaluate(RUN, [PLAN, SAMPLES, REFS]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("qa", "qb"):
                assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            kept = pg.evaluate(KEPT)
            for name in ("qa.json", "qb.json"):
                assert name in kept, (name, list(kept))
                nat = (run / "out" / name).read_text(); web = kept[name]
                at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert nat == web and len(nat) > 100, (name, "first difference at", at, len(nat), len(web), nat[max(0, at - 100):at + 100], web[max(0, at - 100):at + 100])
            corr = pg.evaluate(TABLE)
            nat = (run / "out" / "qb.mzfeat").read_bytes()
            assert corr == hashlib.sha256(nat).hexdigest(), (corr, len(nat))
        step("the massacc/1 of both steps and the corrected .mzfeat of the browser are the files of the native flow run (same SHA-256)", first)

        def again():
            r = pg.evaluate(RUN, [PLAN, SAMPLES, REFS]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in ("qa", "qb") and e["stato"] == "running"], r["ev"]
            miss = pg.evaluate("window.MZLAB_LR_MISS || []"); assert not miss, miss
        step("a second run takes the files from OPFS", again)

        def view():
            pg.evaluate(VIEW)
            pg.wait_for_selector("#qcview svg.flx-sticks", timeout=20000)
            n = pg.locator("#qcview svg.flx-sticks").count(); assert n == 2, ("charts", n, pg.evaluate("document.getElementById('qcview').innerText.slice(0,600)"))
            n = pg.locator("#qcview table").count(); assert n >= 1, ("tables", n)
            small = pg.evaluate("[...document.querySelectorAll('#qcview button')].filter(b => { const r = b.getBoundingClientRect(); return r.width < 24 || r.height < 24; }).map(b => (b.getAttribute('aria-label') || b.className) + ' ' + Math.round(b.getBoundingClientRect().width) + 'x' + Math.round(b.getBoundingClientRect().height))"); assert not small, small
            if AXE:
                pg.add_script_tag(path=AXE)
                v = pg.evaluate("axe.run('#qcview', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact]))"); assert not v, v
        step("the view draws the table and the two charts; targets of 24 px; axe has no serious or critical violation", view)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
