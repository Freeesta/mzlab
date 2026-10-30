"""Flow template «Unit-resolution feature table» (flow/lazy/flow-names.js, id lrtable): two synthetic unit-resolution full-scan files (B_FullMass-t0, -t30) run in «This browser» and the CSV of
the template equals the one written by `mzlab-cli flow run` on the same plan, byte for byte. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_lr_template.py"""
import json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

try: import numpy as np
except ImportError: print("SKIP numpy is needed for the synthetic files"); sys.exit(0)
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
import dati_sintetici as ds

NAMES = ["B_FullMass-t0.mzML", "B_FullMass-t30.mzML"]
def sample(n, t):
    return {"composto": "", "concentrazione": None, "file": n, "istd": False, "note": "", "ordine_iniezione": NAMES.index(n) + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": "t0" if t == 0 else "sample", "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, t) for n, t in zip(NAMES, (0, 30))], "schema": "campioni/2"}, separators=(",", ":"))

tmp = Path(tempfile.mkdtemp(prefix="flow_lr_tpl_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in zip(NAMES, (0, 30)): ds.full_scan(run / n, t, np.random.default_rng(5 + t))

# the plan of the template, the files of the series as the source of the first step, the sample list written by hand
BUILD = """(names) => {
  const S = document.querySelector('.flx-list').__flow, p = FLOW.exportPlan(FLOW.templatePlan(S.reg, FLOW.TEMPLATES.find(t => t.id === 'lrtable')));
  const mz = p.inputs.find(i => i.type === 'mzml' || i.type === 'raw'), sm = p.inputs.find(i => i.type === 'samples');
  p.steps.forEach(s => { for (const [k, v] of Object.entries(s.in)) { if ([].concat(v)[0] === 'file:' + mz.name) s.in[k] = names.map(n => 'file:' + n); } });
  p.inputs = names.map(n => ({ name: n, sha256: '', type: mz.type })).concat(p.inputs.filter(i => i !== mz));
  return { plan: p, samples: sm.name };
}"""
RUN = """async ([plan, samplesName, samples]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson[samplesName] = samples;
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
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.QTOOLS", timeout=120000)
        lib.stage(pg, [run / n for n in NAMES]); lib.ready(pg)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def card():
            (None if pg.locator('.flx-tpl.show').count() else pg.click('button[aria-label="Start from a template"]'))
            c = pg.locator('.flx-tc[data-tpl="lrtable"]'); assert c.count() == 1
            assert "Unit resolution: m/z values are nominal" in c.inner_text() and c.inner_text().count("Good for:") == 1, c.inner_text()
            c.click()
            assert pg.locator("#flx-root .flx-issues:not([hidden])").count() == 0, "the template must check clean"
        step("the template card carries the honest note; clicking it gives a plan with no issue", card)

        built = pg.evaluate(BUILD, NAMES); plan = built["plan"]
        (run / "plan.json").write_text(json.dumps(plan)); (run / built["samples"]).write_text(SAMPLES); out = run / "out"
        nat = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(out)], cwd=ROOT, capture_output=True, text=True)
        assert nat.returncode == 0, nat.stderr[-1500:]

        def first():
            r = pg.evaluate(RUN, [plan, built["samples"], SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            csv = next(s["id"] for s in plan["steps"] if s["step"].startswith("export.table_csv"))
            assert any(e["passo"] == csv and e["stato"] == "done" for e in r["ev"]), r["ev"][-6:]
            kept = pg.evaluate(KEPT); name = csv + ".csv"
            assert name in kept, (name, list(kept))
            nat_csv = (out / name).read_text(); web = kept[name]
            at = next((i for i, (x, y) in enumerate(zip(nat_csv, web)) if x != y), min(len(nat_csv), len(web)))
            assert nat_csv == web and len(nat_csv) > 100, (name, "first difference at", at, len(nat_csv), len(web))
        step("two unit-resolution files: the CSV of the browser is the file of flow run, byte for byte", first)
        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
