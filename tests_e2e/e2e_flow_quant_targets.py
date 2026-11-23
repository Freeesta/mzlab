"""Flow «This browser» quant.targets@1 (flow/lazy/flow-web-quanttargets.js, `flow_quant_targets` of the `quanttargets` .wasm): over the three synthetic unit-resolution files of
tools/lr_synthetic.py the browser writes the long table and the log that `mzlab-cli flow run` writes of the same plan, byte for byte; the module is fetched only when the step runs;
a second run takes both texts from OPFS. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen, python3 and numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_quant_targets.py"""

import json, os, shutil, subprocess, sys, tempfile, threading, urllib.request
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
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed by tools/lr_synthetic.py"); sys.exit(0)

# names that sort like the plan: the browser lists the indexes of OPFS by name, `flow run` takes the order of the plan
STEMS = ["LR_bench-blank-s1", "LR_bench-noise-s1", "LR_bench-s1"]
PLAN = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in STEMS] + [{"name": "targets.csv", "sha256": "", "type": "report"}],
        "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in STEMS]}},
                  {"id": "qt", "step": "quant.targets@1", "in": {"raw": ["idx.index"], "targets": "file:targets.csv"}, "params": {"blank_pattern": "LR_bench-blank*"}}]}

tmp = Path(tempfile.mkdtemp(prefix="flow_quant_targets_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
import lr_synthetic
lr_synthetic.make(run, 1)
truth = json.loads((run / "LR_bench-s1_truth.json").read_text(encoding="utf-8"))
TARGETS = "id,mz,rt_min,rt_max\n" + "".join(f"c{c['id']},{c['mz']},{c['rt_s'] / 60 - 0.3:.3f},{c['rt_s'] / 60 + 0.3:.3f}\n" for c in truth["compounds"][:12])
(run / "plan.json").write_text(json.dumps(PLAN)); (run / "targets.csv").write_text(TARGETS)
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, check=True, capture_output=True)
data = site / "static" / "dati_flow_qt"; data.mkdir()
for n in STEMS: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_qt/" + n + ".mzidx")).body.pipeTo(w); }
}"""
RUN = """async ([plan, targets]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['targets.csv'] = targets;
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
# the texts the run kept in OPFS (mzlab/flow/<name>), by name
KEPT = """async (names) => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), out = {};
  for (const n of names) { try { out[n] = await (await (await d.getFileHandle(n)).getFile()).text(); } catch (_) { /* not kept */ } }
  return out;
}"""
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs, asked = [], []
URL = f"http://127.0.0.1:{PORT}/"
for _ in range(100):
    try: urllib.request.urlopen(URL, timeout=2); break
    except OSError: threading.Event().wait(0.2)
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.bar.hint','1');localStorage.setItem('mzlab.panels.side_by_side.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
        pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
        pg.on("request", lambda r: asked.append(r.url) if "/quanttargets/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, STEMS)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        assert not asked, ("the quanttargets module was fetched before the step ran", asked)

        def first():
            r = pg.evaluate(RUN, [PLAN, TARGETS]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert any(e["passo"] == "qt" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), r["ev"][-6:]
            assert asked, "the quanttargets module was not fetched when quant.targets ran"
            kept = pg.evaluate(KEPT, ["qt.csv", "qt.log.json"])
            for name in ("qt.csv", "qt.log.json"):
                assert name in kept, (name, list(kept))
                nat = (run / "out" / name).read_text(); web = kept[name]
                at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert nat == web and len(nat) > 100, (name, "first difference at", at, len(nat), len(web), nat[max(0, at - 100):at + 100], web[max(0, at - 100):at + 100])
            assert ",detected," in kept["qt.csv"], kept["qt.csv"][:400]
        step("the table and the log of the browser are the files of the native flow run, byte for byte", first)

        def again():
            r = pg.evaluate(RUN, [PLAN, TARGETS]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] == "qt" and e["stato"] == "running"], r["ev"]
        step("a second run takes both texts from OPFS", again)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
