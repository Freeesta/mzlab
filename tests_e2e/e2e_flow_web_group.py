"""Flow «This browser» runs features.group@1 (grouping of a per-ion table into compounds, module `flowtab`): six synthetic files where four compounds each give [M+H]+, its 13C
isotope and [M+Na]+, with heights that change from file to file. The plan detect → align (`mode=ion`) → gapfill → features.group goes to `mzlab-cli flow run` (rc 0), and the ionnet/1
text the browser keeps is the native file byte for byte (same SHA-256); a second run takes it from OPFS, the `flowtab` .wasm is fetched only when features.group runs, and the
«Untargeted analysis» template aligns by ion and wires features.group to the gap-filled table. features.group on a table.filter view of that table (the table the view
stands for, `flow_table_of_view`) is the native file of the same plan too. Builds the site and the .wasm (cargo, wasm-bindgen, numpy: SKIP without).
Run from the repo root: python3 tests_e2e/e2e_flow_web_group.py"""
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
except ImportError: print("SKIP numpy is needed for the synthetic files"); sys.exit(0)
if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
import dati_sintetici as ds
from valida import sintetici_mzml as sm

PROTON, NA, C13 = 1.007276, 22.989218, 1.003355
NAMES = [f"GR_{k:02d}" for k in range(6)]
COMPOUNDS = [(300.1100, 1.0), (356.2000, 1.6), (414.3000, 2.2), (522.4400, 2.8)]   # neutral mass, RT (min)
GAIN = np.random.default_rng(7).uniform(0.3, 2.0, (len(NAMES), len(COMPOUNDS)))  # each compound rises and falls on its own over the files
CYCLE, RT_FROM, RT_TO, SIGMA = 0.01, 0.6, 3.4, 0.04

def ions(m):
    """The three ions of one compound: m/z and height at gain 1."""
    return [(m + PROTON, 1.0e6), (m + PROTON + C13, 2.0e5), (m + NA, 4.0e5)]

def write_mzml(path, k, seed):
    """One file: MS1 centroids every CYCLE min, the ions of the four compounds times the gain of the file, and a little noise."""
    rng = np.random.default_rng(seed)
    rts = np.arange(RT_FROM, RT_TO, CYCLE)
    specs, tic = [], []
    for n, rt in enumerate(rts):
        mz = [rng.uniform(100, 900, 60)]; it = [rng.lognormal(np.log(8e3), 0.7, 60)]
        for c, (m, r0) in enumerate(COMPOUNDS):
            shape = GAIN[k, c] * np.exp(-0.5 * ((rt - r0) / SIGMA) ** 2)
            for x, h in ions(m):
                y = h * shape * rng.normal(1, 0.01)
                if y >= 2e3: mz.append(np.array([x * (1 + rng.normal(0, 0.8e-6))])); it.append(np.array([y]))
        mz, it = np.concatenate(mz), np.concatenate(it); o = np.argsort(mz, kind="stable")
        specs.append(sm._scan(n, rt, 1, mz[o], it[o], filt=sm.FILTER1, res=60000)); tic.append(float(it.sum()))
    chroms = [ds._chrom(0, "TIC", rts, np.array(tic))]
    path.write_text(ds._hr_head("exploris", False, path.stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                    + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")

def sample(k, n):
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": k + 1, "replica": 1, "serie": "", "sha256": "", "tempo": k, "tempo_confermato": False,
            "tempo_min": k, "tipo": "t0" if k == 0 else "sample", "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
SAMPLES = json.dumps({"campioni": [sample(k, n) for k, n in enumerate(NAMES)], "schema": "campioni/2"}, separators=(",", ":"))

def plan(group, view=False, params=None):
    """The plan: index → detect → align by ion → gap filling, and (with `group`) the grouping, of the gap-filled table or (with `view`) of a filter view of it, with `params`."""
    inputs = [{"name": n + ".mzML", "sha256": "", "type": "mzml"} for n in NAMES] + ([{"name": "samples.json", "sha256": "", "type": "samples"}] if view else [])
    st = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
          {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
          {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}, "params": {"mode": "ion"}},
          {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}}]
    if view:
        st += [{"id": "flt", "step": "table.filter@2", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": {"min_detected": 4, "normalize": "sum"}}]
    if group:
        st += [{"id": "grpv" if view else "grp", "step": "features.group@1", "in": {"table": "flt.table" if view else "gap.table"}, **({"params": params} if params else {})}]
    return {"format": "mzflow/2", "inputs": inputs, "steps": st}

tmp = Path(tempfile.mkdtemp(prefix="flow_group_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
run = tmp / "run"; run.mkdir()
for i, n in enumerate(NAMES): write_mzml(run / (n + ".mzML"), i, 100 + i)
FEW = {"min_corr_samples": 7}   # more than the six files: no correlation can be tested
(run / "group.json").write_text(json.dumps(plan(True))); (run / "view.json").write_text(json.dumps(plan(True, True))); (run / "samples.json").write_text(SAMPLES)
RAISE = {"ions_pos": "[M+H]+,[M+NH4]+"}   # a run the page refuses at its safe budget and makes again with the high one
(run / "few.json").write_text(json.dumps(plan(True, params=FEW))); (run / "raise.json").write_text(json.dumps(plan(True, params=RAISE)))
out, out_view, out_few, out_raise = run / "out", run / "out_view", run / "out_few", run / "out_raise"
for name, to in (("group.json", out), ("view.json", out_view), ("few.json", out_few), ("raise.json", out_raise)):
    nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / name), "--in", str(run), "--out", str(to)], cwd=ROOT, capture_output=True, text=True)
    assert nat_run.returncode == 0, (name, nat_run.stderr[-1500:])
data = site / "static" / "dati_flow_group"; data.mkdir()
for n in NAMES: shutil.copy2(next(out.glob("*-" + n + ".mzidx")), data / (n + ".mzidx"))
NATIVE, NATIVE_VIEW, NATIVE_FEW, NATIVE_RAISE = (out / "grp.json").read_bytes(), (out_view / "grpv.json").read_bytes(), (out_few / "grp.json").read_bytes(), (out_raise / "grp.json").read_bytes()
assert "warn.flow.group.corrFiles" in (out_few / "prov.json").read_text() and b"MS1 correlation" not in NATIVE_FEW and b"MS1 correlation" in NATIVE

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_group/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# sets the plan and the files of the plan in the open Flow page, runs it, resolves with the events and the error key
RUN = """async ([plan, samples]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples;
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
# the texts of the last run, by name (mzlab/flow/<name>: the newest one of each name; the cache under mzlab/flow/msn/ holds every run, in no order)
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'), out = {};
  for await (const [n, h] of d.entries()) if (h.kind === 'file' && n.endsWith('.json')) out[n] = await (await h.getFile()).text();
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
        pg.on("request", lambda r: asked.append(r.url) if "/flowtab/" in r.url else None)
        pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.QTOOLS", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
        pg.evaluate(PUT, NAMES)

        def upstream():
            r = pg.evaluate(RUN, [plan(False), SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("det", "ali", "gap"): assert any(e["passo"] == i and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            assert not asked, ("the flowtab module was fetched by a plan without features.group", asked)
        step("a plan without features.group does not fetch the flowtab module", upstream)

        def first():
            r = pg.evaluate(RUN, [plan(True), SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert any(e["passo"] == "grp" and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), r["ev"][-6:]
            assert asked, "the flowtab module was not fetched when features.group ran"
            web = pg.evaluate(KEPT).get("grp.json"); assert web is not None
            nat = NATIVE.decode()
            at = next((j for j, (x, y) in enumerate(zip(nat, web)) if x != y), 0)
            assert nat == web, ("first difference at", at, nat[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
            assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE).hexdigest()
            d = json.loads(web); assert d["format"] == "ionnet/1" and d["polarity"] == 1, web[:200]
            kinds = {e["EdgeType"] for e in d["edges"]}
            assert {"MS1 annotation", "MS1 isotope"} <= kinds, (kinds, web[:400])
            assert d["groups"] < d["rows"], (d["groups"], d["rows"])
        step("the browser gives the ionnet/1 of the native flow run, byte for byte", first)

        def again():
            before = len(asked)
            r = pg.evaluate(RUN, [plan(True), SAMPLES]); assert not r.get("err"), r
            assert not [e for e in r["ev"] if e["passo"] == "grp" and e["stato"] == "running"], r["ev"]
            assert any(e["passo"] == "grp" and e["stato"] in ("done", "reused") and e["via"] == "browser" for e in r["ev"]), r["ev"][-6:]
            assert len(asked) == before, "the second run fetched the module again"
        step("a second run takes the text from OPFS", again)

        def view():
            r = pg.evaluate(RUN, [plan(True, True), SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            for i in ("flt", "grpv"): assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            web = pg.evaluate(KEPT).get("grpv.json"); assert web is not None
            assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE_VIEW).hexdigest(), (web[:300], NATIVE_VIEW[:300])
            assert NATIVE_VIEW != NATIVE and json.loads(web)["rows"] < json.loads(NATIVE)["rows"], "the view kept every row: the case proves nothing"
        step("features.group on a table.filter view gives the native ionnet/1 of the same plan", view)

        def few():
            r = pg.evaluate(RUN, [plan(True, params=FEW), SAMPLES]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            ev = [e for e in r["ev"] if e["passo"] == "grp" and e["stato"] == "done"]; assert ev and "min_corr_samples" in (ev[-1].get("note") or ""), r["ev"][-6:]
            web = pg.evaluate(KEPT).get("grp.json"); assert web is not None
            assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE_FEW).hexdigest(), (web[:300], NATIVE_FEW[:300])
            card = pg.evaluate("() => { const v = FLOW.liveView.look(FLOW.liveView.node('grp')); return [v.word, v.tone, v.title]; }")
            assert card[0] == "done, with a note" and card[1] == "warn" and "min_corr_samples" in card[2], card
            # the same plan again: the text comes from OPFS and the note with it
            r = pg.evaluate(RUN, [plan(True, params=FEW), SAMPLES]); assert not r.get("err"), r
            assert "min_corr_samples" in (pg.evaluate("() => FLOW.liveView.node('grp').note") or ""), r["ev"][-4:]
        step("fewer files than min_corr_samples: the native text and a note on the card", few)

        def budget():
            # the tables upstream are reused; only features.group runs again, in a budget that cannot hold it
            pg.evaluate("() => { window.__mb = window.MZLAB_RUST.budgetMb; window.MZLAB_RUST.budgetMb = 0.0001; }")
            try: r = pg.evaluate(RUN, [plan(True, params={"min_corr": 0.8}), SAMPLES])
            finally: pg.evaluate("() => { window.MZLAB_RUST.budgetMb = window.__mb; }")
            assert any(e["passo"] == "grp" and e["stato"] in ("error", "refused") for e in r["ev"]), r["ev"][-6:]
            err = pg.evaluate("() => FLOW.liveView.look(FLOW.liveView.node('grp')).title")
            assert "Not enough memory for the «group» phase" in err, err
        step("past the budget features.group is refused as the alignment is", budget)

        # a safe budget that cannot hold the grouping and a high one that can (the levels of the engine's choice): Run asks once, the yes runs it again with the high budget
        LEVELS = "() => { const r = window.MZLAB_RUST; window.__lv = [r.budgetMb, r.choice]; r.budgetMb = 0; r.choice = { ...(r.choice || {}), mb: 0.0001, high_mb: 4096 }; }"
        UNDO = "() => { const r = window.MZLAB_RUST; [r.budgetMb, r.choice] = window.__lv; }"

        def raise_yes():
            pg.evaluate(LEVELS)
            try:
                pg.evaluate("([plan, samples]) => { window.__run = (" + RUN + ")([plan, samples]); }", [plan(True, params=RAISE), SAMPLES])
                pg.wait_for_selector("#memmore", timeout=120000)
                text = pg.inner_text("#memmore #mm-p")
                assert "ion grouping" in text or "raggruppamento degli ioni" in text, text
                card = pg.evaluate("() => FLOW.liveView.look(FLOW.liveView.node('grp')).word")
                assert card == "Allow more memory: Run now", card
                pg.click("#memmore #mm-yes")
                r = pg.evaluate("() => window.__run")
            finally: pg.evaluate(UNDO)
            assert not r.get("err"), (r.get("err"), r["ev"][-6:])
            web = pg.evaluate(KEPT).get("grp.json"); assert web is not None
            assert hashlib.sha256(web.encode()).hexdigest() == hashlib.sha256(NATIVE_RAISE).hexdigest(), (web[:300], NATIVE_RAISE[:300])
            assert NATIVE_RAISE != NATIVE, "the raised run gave the text of the default run: the case proves nothing"
        step("past the safe budget and within the high one, Run asks and the yes gives the native ionnet/1", raise_yes)

        def raise_no():
            pg.evaluate(LEVELS); pg.evaluate("() => { window.__ask = window.mzlabAskMore; window.__asked = []; window.mzlabAskMore = async i => { window.__asked.push(i); return false; }; }")
            try: r = pg.evaluate(RUN, [plan(True, params={"min_corr": 0.9}), SAMPLES])
            finally: pg.evaluate(UNDO); pg.evaluate("() => { window.mzlabAskMore = window.__ask; }")
            asked = pg.evaluate("window.__asked")
            assert len(asked) == 1 and asked[0]["fase"] == "group" and asked[0]["high_mb"] == 4096 and asked[0]["stima_mb"] > 0, asked
            assert r.get("err") and r["err"]["key"] == "error.featMemory", r.get("err")
            err = pg.evaluate("() => FLOW.liveView.look(FLOW.liveView.node('grp')).title")
            assert "Not enough memory for the «group» phase" in err, err
        step("a no keeps features.group refused, as for the alignment", raise_no)

        def template():
            got = pg.evaluate("""() => { const S = document.querySelector('.flx-list').__flow, F = FLOW;
              const plan = F.templatePlan(S.reg, F.TEMPLATES.find(t => t.id === 'identify'));
              const ali = plan.steps.find(s => s.step.startsWith('features.align@')), grp = plan.steps.find(s => s.step.startsWith('features.group@')),
                    gap = plan.steps.find(s => s.step.startsWith('features.gapfill@'));
              return { mode: ali && ali.params && ali.params.mode, table: grp && grp.in.table, gap: gap && gap.id }; }""")
            assert got["mode"] == "ion", got
            assert got["table"] in (got["gap"] + ".table", [got["gap"] + ".table"]), got
        step("the Untargeted analysis template aligns by ion and groups the gap-filled table", template)

        step("no request was left unserved by the engine", lambda: (lambda m: m == [] or (_ for _ in ()).throw(AssertionError(m)))(pg.evaluate("window.MZLAB_LR_MISS || []")))
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
