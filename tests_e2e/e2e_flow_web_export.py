"""Flow «This browser» export.table_csv, export.mztab and table.filter (flow-live-jobs.js `tableExport`, `tableFilter`, `export_table_auto` and `table_view_auto` of the `table` .wasm): over the five demo full-scan files the browser writes the CSV (full and
FBMN quant), the mzTab-M and the filter view that `mzlab-cli flow run` writes of the same plan, byte for byte; a second run takes them from OPFS; a plan with two branches (a preset and a gap filling on one) gives the CSV of `flow run` for each. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen and python3: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web_export.py"""
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

# the demo files under names that sort like the plan: the browser lists the indexes of OPFS by name, `flow run` takes the order of the plan
NAMES = ["FullScan_t00", "FullScan_t05", "FullScan_t10", "FullScan_t15", "FullScan_t30"]
TIMES = [0, 5, 10, 15, 30]
def sample(n, t):
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": NAMES.index(n) + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": "t0" if t == 0 else "sample", "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(n, t) for n, t in zip(NAMES, TIMES)], "schema": "campioni/2"}, separators=(",", ":"))
PLAN = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": "samples.json", "sha256": "", "type": "samples"}, ],
        "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
                  {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                  {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                  {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
                  {"id": "csv", "step": "export.table_csv@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
                  {"id": "quant", "step": "export.table_csv@1", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": {"kind": "quant"}},
                  {"id": "mzt", "step": "export.mztab@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
                  {"id": "flt", "step": "table.filter@1", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": {"min_detected": 3, "normalize": "sum"}},
                  {"id": "flt2", "step": "table.filter@1", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": {"min_replicate_fraction": 0.5, "normalize": "istd", "istd_id": 1}}]}

# two branches over the same indexes, one with a preset and a gap filling, the other with its own detection parameters: each table takes only its own line
BRANCHES = {"format": "mzflow/2", "inputs": PLAN["inputs"], "steps": [PLAN["steps"][0],
            {"id": "detb", "step": "features.detect@1", "in": {"index": ["idx.index"]}, "params": {"min_points": 3}},
            {"id": "deta", "step": "features.detect@1", "preset": "hr", "in": {"index": ["idx.index"]}},
            {"id": "alia", "step": "features.align@1", "in": {"peaks": ["deta.peaks"]}},
            {"id": "alib", "step": "features.align@1", "in": {"peaks": ["detb.peaks"]}, "params": {"rt_gap": 0.05}},
            {"id": "gapa", "step": "features.gapfill@1", "in": {"table": "alia.table", "index": ["idx.index"]}},
            {"id": "csva", "step": "export.table_csv@1", "in": {"table": "gapa.table", "samples": "file:samples.json"}},
            {"id": "csvb", "step": "export.table_csv@1", "in": {"table": "alib.table", "samples": "file:samples.json"}}]}
tmp = Path(tempfile.mkdtemp(prefix="flow_web_export_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for n, t in zip(NAMES, TIMES): shutil.copy2(ROOT / "mzlab" / "web" / "esempi" / ("FullScan_t%d.mzML" % t), run / (n + ".mzML"))
(run / "plan.json").write_text(json.dumps(PLAN)); (run / "samples.json").write_text(SAMPLES)
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, check=True, capture_output=True)
(run / "branches.json").write_text(json.dumps(BRANCHES))
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "branches.json"), "--in", str(run), "--out", str(run / "out_b")], cwd=ROOT, check=True, capture_output=True)
print("native outputs:", sorted(x.name for x in (run / "out").iterdir() if not x.name.endswith(".mzidx")))
data = site / "static" / "dati_flow_exp"; data.mkdir()
for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_exp/" + n + ".mzidx")).body.pipeTo(w); }
}"""
RUN = """async ([plan, samples]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples;
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
NODES = """async () => (await (await fetch('api/flow/sched/nodes', { method: 'POST', body: '{}' })).json()).map(n => [n.instance, n.item, n.state, n.error_key || n.by || ''])"""
# the kept files of mzlab/flow/msn/ and the keys index.json lists: the same set (jobs that end together must not drop each other's key)
LISTED = """async () => {
  const msn = await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow')).getDirectoryHandle('msn'), files = [];
  for await (const n of msn.keys()) if (n !== 'index.json') files.push(n.replace(/\\.json$/, ''));
  const list = JSON.parse(await (await (await msn.getFileHandle('index.json')).getFile()).text());
  return [files.sort(), [...list].sort()];
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
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def first():
            r = pg.evaluate(RUN, [PLAN, SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("csv", "quant", "mzt", "flt", "flt2"):
                assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            kept = pg.evaluate(KEPT)
            for name, nat_name in (("csv.csv", "csv.csv"), ("quant.csv", "quant.csv"), ("mzt.mzTab", "mzt.mzTab"), ("flt.json", "flt.json"), ("flt2.json", "flt2.json")):
                assert name in kept, (name, list(kept))
                nat = (run / "out" / nat_name).read_text(); web = kept[name]
                at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert nat == web and len(nat) > 100, (name, "first difference at", at, len(nat), len(web), nat[max(0, at - 100):at + 100], web[max(0, at - 100):at + 100])
            files, listed = pg.evaluate(LISTED)
            assert files == listed and len(files) >= 5, (files, listed)
        step("the CSV, the quant CSV, the mzTab-M and the filter views (the second with the replicate fraction and the internal standard) of the browser are the files of the native flow run", first)

        def again():
            r = pg.evaluate(RUN, [PLAN, SAMPLES]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] in ("csv", "quant", "mzt", "flt", "flt2") and e["stato"] == "running"], r["ev"]
        step("a second run takes the files from OPFS", again)

        def branches():
            r = pg.evaluate(RUN, [BRANCHES, SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            kept = pg.evaluate(KEPT)
            for name in ("csva.csv", "csvb.csv"):
                assert name in kept, (name, list(kept), [(e["passo"], e["stato"], e.get("modo")) for e in r["ev"] if e["passo"] in ("deta", "alia", "gapa", "csva")], pg.evaluate(NODES))
                nat = (run / "out_b" / name).read_text(); web = kept[name]
                at = next((i for i, (x, y) in enumerate(zip(nat, web)) if x != y), min(len(nat), len(web)))
                assert nat == web and len(nat) > 100, (name, "first difference at", at, len(nat), len(web), nat[max(0, at - 100):at + 100], web[max(0, at - 100):at + 100])
            assert kept["csva.csv"] != kept["csvb.csv"]
        step("a plan with two branches (a preset and a gap filling on one, other detection parameters on the other) gives the CSV of the native flow run for each", branches)

        # the memory of the whole read of the .mzfeat (flow-live-jobs.js `wholeRead`): the kept view of `flt` is dropped so table.filter runs again, under a safe budget of 8 MB
        DROP = """async () => { const msn = await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow')).getDirectoryHandle('msn');
          for await (const [n, h] of msn.entries()) if (n !== 'index.json' && JSON.parse(await (await h.getFile()).text()).name === 'flt.json') await msn.removeEntry(n); }"""
        LEVELS = "(high) => { const r = window.MZLAB_RUST; window.__lv = [r.budgetMb, r.choice]; r.budgetMb = 0; r.choice = { ...(r.choice || {}), mb: 8, high_mb: high }; }"
        UNDO = "() => { const r = window.MZLAB_RUST; [r.budgetMb, r.choice] = window.__lv; }"

        def read_refused():
            pg.evaluate(DROP); pg.evaluate(LEVELS, 8)
            # another min_detected than the plan's: a refused job is not tried again by Run under the same plan (read_raised runs the plan itself)
            other = json.loads(json.dumps(PLAN)); next(s for s in other["steps"] if s["id"] == "flt")["params"]["min_detected"] = 2
            try: r = pg.evaluate(RUN, [other, SAMPLES])
            finally: pg.evaluate(UNDO)
            assert any(e["passo"] == "flt" and e["stato"] in ("error", "refused") for e in r["ev"]), r["ev"][-6:]
            title = pg.evaluate("() => FLOW.liveView.look(FLOW.liveView.node('flt')).title")
            assert ("Reading the feature table of this step whole would need about" in title or "Leggere per intero la tabella delle feature" in title) and "8 MB" in title, title
            assert "flt.json" not in pg.evaluate(KEPT)
        step("past the budget and the high level, table.filter is refused before the .mzfeat is read, with the numbers of the read", read_refused)

        def read_raised():
            pg.evaluate(DROP); pg.evaluate(LEVELS, 4096)
            try:
                pg.evaluate("([plan, samples]) => { window.__run = (" + RUN + ")([plan, samples]); }", [PLAN, SAMPLES])
                pg.wait_for_selector("#memmore", timeout=120000)
                text = pg.inner_text("#memmore #mm-p")
                assert "reading of the feature table" in text or "lettura della tabella delle feature" in text, text
                pg.click("#memmore #mm-yes")
                r = pg.evaluate("() => window.__run")
            finally: pg.evaluate(UNDO)
            assert not r.get("err"), (r.get("err"), r["ev"][-6:])
            assert pg.evaluate(KEPT).get("flt.json") == (run / "out" / "flt.json").read_text()
        step("past the safe budget and within the high one, Run asks once and the yes gives the view of the native flow run", read_raised)

        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
