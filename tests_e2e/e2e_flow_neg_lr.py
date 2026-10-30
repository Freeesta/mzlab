"""Flow «This browser» against `mzlab-cli flow run` in both polarities and both resolutions (docs/studi/MATRICE_NEG_LR.md): a small invented bentazone series (the fixture of
crates/mzlab-cli/src/flow_bentazone_tests.rs, ported here: a parent that fades, two products that grow) in HR+, HR-, LR+ and LR-, through the base chain, tp.generate, tp.link, tp.filter,
tp.network, table.filter, export.table_csv, export.mztab, tp.position at high resolution and tp.dig_lr at unit resolution. Every text the browser keeps equals the file of the native
run, byte for byte; the check of the page and of `flow run` take the facets from the files and refuse the same steps (formula.ms1 and tp.position at unit resolution, tp.dig_lr at high).
A series that mixes polarities (mix-pol) or resolutions (mix-res) is refused by features.align with the same error key and parameters on both sides.
Builds the site and the .wasm of mzFinder (needs cargo and wasm-bindgen: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_neg_lr.py [hr+|hr-|lr+|lr-|mix-pol|mix-res]...
NEG_LR_KEEP=<dir> keeps both sides' files there; the plan leaves the polarity and the unit mass to the files (both sides read them from the indexes); NEG_LR_WRITTEN=1 writes them."""
import base64, json, math, os, shutil, struct, subprocess, sys, tempfile, threading, urllib.request
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

CASES = {"hr+": (1, True), "hr-": (-1, True), "lr+": (1, False), "lr-": (-1, False)}
# (polarity, high resolution) of file k: two negative and three positive files, two unit and three high-resolution ones
MIXED = {"mix-pol": lambda k: (-1 if k < 2 else 1, True), "mix-res": lambda k: (1, k >= 2)}
CHOSEN = [a for a in sys.argv[1:] if a in CASES or a in MIXED] or list(CASES) + list(MIXED)
PROTON = 1.007276
NAMES = ["BEN_blank", "BEN_T00", "BEN_T2p5", "BEN_T5", "BEN_T10p25"]
TIMES = [None, 0, 2.5, 5, 10.25]
HEIGHTS = [[0, 0, 0], [1e6, 0, 0], [7e5, 2e5, 1e5], [4e5, 4e5, 2e5], [1e5, 3e5, 3e5]]
ION = [(240.0568, 5.0), (256.0517, 4.3), (198.0098, 3.6)]
PARENT = json.dumps({"name": "bentazone", "formula": "C10H12N2O3S", "smiles": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"})

def rnd(seed):
    seed[0] = (seed[0] * 6364136223846793005 + 1442695040888963407) % (1 << 64)
    return (seed[0] >> 11) / float(1 << 53)

def array(acc, v):
    b = base64.b64encode(struct.pack("<%dd" % len(v), *v)).decode()
    return ('<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>'
            f'<cvParam cvRef="MS" accession="{acc}" name=""/><binary>{b}</binary></binaryDataArray>')

def mzml(k, pol, hr):
    """One centroid mzML of 400 MS1 scans: the analyzer of the filter (FTMS / ITMS) and a tenth of a Da of error make the unit-resolution files."""
    seed = [7 + k]; acc, sign = ("MS:1000129", "-") if pol < 0 else ("MS:1000130", "+")
    filt = f"{'FTMS' if hr else 'ITMS'} {sign} {'p' if hr else 'c'} ESI Full ms [50.00-500.00]"
    out = ['<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="400">']
    for i in range(400):
        rt = i * 0.02; pk = []
        def add(m, h):
            pk.append((m + (0.0 if hr else 0.05 + 0.04 * (rnd(seed) - 0.5)), h))
        for j, (neutral, t) in enumerate(ION):
            h = HEIGHTS[k][j] * math.exp(-((rt - t) / 0.08) ** 2 / 2)
            if h < 1e3: continue
            ion = neutral + pol * PROTON
            add(ion, h); add(ion + 1.00335, h * 0.14); add(ion + 1.9958, h * 0.06)
        add(300.1234, 4e5 * math.exp(-((rt - 2.0) / 0.1) ** 2 / 2) + 1e3)
        for _ in range(10):
            m = 100 + 300 * rnd(seed); add(m, 500 + 1500 * rnd(seed))
        pk.sort()
        out.append(f'<spectrum index="{i}" id="scan={i + 1}" defaultArrayLength="{len(pk)}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
                   f'<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="{acc}" name="scan"/>'
                   f'<cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="{filt}"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" '
                   f'name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">'
                   + array("MS:1000514", [p[0] for p in pk]) + array("MS:1000515", [p[1] for p in pk]) + "</binaryDataArrayList></spectrum>")
    return "".join(out) + "</spectrumList></run></mzML>"

def sample(n, t):
    kind = "blank" if t is None else "t0" if t == 0 else "sample"
    return {"composto": "", "concentrazione": None, "file": n + ".mzML", "istd": False, "note": "", "ordine_iniezione": NAMES.index(n) + 1, "replica": 1, "serie": "", "sha256": "", "tempo": t, "tempo_confermato": False,
            "tempo_min": t, "tipo": kind, "tipo_bianco": None, "unita": None, "unita_tempo": "min"}
# the series in the order of injection: the browser runner lists the indexes of OPFS by name, and every step puts the samples in the order of the files
# of the table by name (`campioni::order_like`), so both sides give the same files whatever the order
ORDER = NAMES
SAMPLES = json.dumps({"campioni": [sample(n, TIMES[NAMES.index(n)]) for n in ORDER], "schema": "campioni/2"}, separators=(",", ":"))

# the plan leaves the polarity and the unit mass unwritten: the browser runner and `flow run` take them from the indexes (`mzlab_feat::nature`);
# NEG_LR_WRITTEN=1 runs the variant that writes them, as the page does from the files open in Data (`inheritNature`)
WRITTEN = os.environ.get("NEG_LR_WRITTEN") == "1"
def plan(pol, hr):
    nat = {"gen": {"polarity": pol, "nominal": not hr}, "lnk": {"polarity": pol}} if WRITTEN else {}
    s = [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in ORDER]}},
         {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
         {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
         {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
         {"id": "gen", "step": "tp.generate@1", "in": {"parent": "file:parent.json", "index": ["idx.index"]}},
         {"id": "lnk", "step": "tp.link@2", "in": {"table": "gap.table", "samples": "file:samples.json", "parent": "file:parent.json", "index": ["idx.index"], "hypotheses": "gen.hypotheses"}},
         {"id": "flt", "step": "tp.filter@2", "in": {"tp": "lnk.tp", "hypotheses": "gen.hypotheses"}},
         {"id": "net", "step": "tp.network@1", "in": {"tp": "lnk.tp", "table": "lnk.tp", "samples": "file:samples.json"}},
         {"id": "fil", "step": "table.filter@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
         {"id": "fdu", "step": "table.filter@1", "in": {"table": "gap.table", "samples": "file:samples.json"}, "params": {"dup_ppm": 10} if hr else {"dup_da": 0.3}},
         {"id": "csv", "step": "export.table_csv@1", "in": {"table": "gap.table", "samples": "file:samples.json"}},
         {"id": "mzt", "step": "export.mztab@1", "in": {"table": "gap.table", "samples": "file:samples.json"}}]
    s.append(POS if hr else {"id": "dig", "step": "tp.dig_lr@1", "in": {"table": "gap.table", "samples": "file:samples.json", "parent": "file:parent.json", "index": ["idx.index"]}})
    for x in s:
        if x["id"] in nat: x["params"] = nat[x["id"]]
    return {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in ORDER] + [{"name": "samples.json", "sha256": "", "type": "samples"}, {"name": "parent.json", "sha256": "", "type": "structure"}], "steps": s}
def refusal(v):
    """The first {error_key, params} of the native prov/1 whose key is a refusal of the alignment."""
    if isinstance(v, dict):
        if str(v.get("error_key", "")).startswith("err.flow.align."): return {"key": v["error_key"], "params": v.get("params", {})}
        v = list(v.values())
    if isinstance(v, list):
        return next((r for r in map(refusal, v) if r), None)
    return None
POS = {"id": "pos", "step": "tp.position@1", "in": {"tp": "lnk.tp", "parent": "file:parent.json"}}   # high resolution only: fragments at 5 ppm
TEXT_IDS = ["gen", "flt", "pos", "net", "fil", "fdu", "csv", "mzt", "dig"]

def check_plan(pol, hr):
    """The plan of `plan` with the steps the facets of the files refuse: formula.ms1 and tp.position (high resolution only) or, on high-resolution files, tp.dig_lr."""
    pl = plan(pol, hr); src = ["file:" + n + ".mzML" for n in ORDER]
    pl["steps"] += [{"id": "t", "step": "features.table@1", "in": {"source": src}}, {"id": "f1", "step": "formula.ms1@1", "in": {"features": "t.report", "index": src}}]
    pl["steps"].append(POS if not hr else {"id": "dig", "step": "tp.dig_lr@1", "in": {"table": "gap.table", "samples": "file:samples.json", "parent": "file:parent.json", "index": ["idx.index"]}})
    return pl
# the check of the page once it has the facets of the indexes of OPFS (`series_estimates` of the proofs .wasm asked for `?facets`): the facet problems with their text
CHECK = """async ([plan]) => {
  for (let i = 0; i < 600 && !FLOW.fileFacets; i++) await new Promise(r => setTimeout(r, 100));
  const S = document.querySelector('.flx-list').__flow;
  return { facets: FLOW.fileFacets, problems: FLOW.validate(plan, S.reg).map(p => ({ at: p.instance + '.' + p.port + ':' + p.key, params: p.params, text: FLOW.message(p) })) };
}"""

tmp = Path(tempfile.mkdtemp(prefix="flow_neg_lr_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)

PUT = """async ([names, dir]) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/" + dir + "/" + n + ".mzidx")).body.pipeTo(w); }
}"""
RUN = """async ([plan, samples, parent]) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['samples.json'] = samples;
  await FLOW.parentSet(S, S.plan.inputs.find(i => i.name === 'parent.json'), parent);
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
        b = p.chromium.launch()
        for case in CHOSEN:
            of = MIXED.get(case) or (lambda k, c=CASES[case]: c)
            pol, hr = of(len(NAMES) - 1); tag = case.replace("+", "p").replace("-", "n")
            run = tmp / ("run_" + tag); run.mkdir()
            for k, n in enumerate(NAMES): (run / (n + ".mzML")).write_text(mzml(k, *of(k)))
            pl = plan(pol, hr)
            if case in MIXED: pl["steps"] = pl["steps"][:4]
            (run / "plan.json").write_text(json.dumps(pl)); (run / "samples.json").write_text(SAMPLES); (run / "parent.json").write_text(PARENT)
            nat_run = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "plan.json"), "--in", str(run), "--out", str(run / "out")], cwd=ROOT, capture_output=True, text=True)
            nat_err = None
            if case in MIXED:
                nat_err = refusal(json.loads((run / "out" / "prov.json").read_text())) if (run / "out" / "prov.json").exists() else None
                if not nat_err:
                    steps.append((case + ": native flow run", "FAIL no err.flow.align.* " + nat_run.stderr[-900:])); continue
            elif nat_run.returncode != 0:
                steps.append((case + ": native flow run", "FAIL " + nat_run.stderr[-900:])); continue
            data = site / "static" / ("dati_neg_lr_" + tag); data.mkdir()
            for n in NAMES: shutil.copy2(run / "out" / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))
            # a fresh page per case: the texts kept in OPFS are named by the ids of the steps, which repeat from one case to the other
            pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
            pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.on("pageerror", lambda e: errs.append(str(e) + " " + str(getattr(e, "stack", ""))[:400]))
            pg.on("console", lambda m: errs.append(m.text) if m.type == "error" and "GL Driver" not in m.text else None)
            pg.goto(URL); pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_function("window.QTOOLS", timeout=120000)
            pg.evaluate(PUT, [NAMES, data.name])
            pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
            pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)
            ids = [s["id"] for s in pl["steps"] if s["id"] in TEXT_IDS]

            def refused(pl=pl, nat_err=nat_err):
                r = pg.evaluate(RUN, [pl, SAMPLES, PARENT]); e = r.get("err") or {}
                assert {"key": e.get("key"), "params": e.get("params")} == nat_err, (e, nat_err, r["ev"][-4:])
            if case in MIXED:
                step(case + ": the browser refuses the alignment as the native flow run does", refused); pg.context.close(); continue
            def facets(run=run, pol=pol, hr=hr):
                cp = check_plan(pol, hr); (run / "check.json").write_text(json.dumps(cp))
                nat = subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / "check.json"), "--in", str(run), "--dry-run", "--json"], cwd=ROOT, capture_output=True, text=True)
                native = [{"at": p["instance"] + "." + p["port"] + ":" + p["error_key"], "params": p["params"]} for p in json.loads(nat.stdout).get("problems", [])]
                r = pg.evaluate(CHECK, [cp]); res = "hr" if hr else "lr"
                assert r["facets"] == {"resolution": [res], "polarity": [pol]}, r["facets"]
                web = [{"at": p["at"], "params": p["params"]} for p in r["problems"]]
                assert web == native, (web, native)
                want = ["dig.table:err.flow.facet", "dig.index:err.flow.facet"] if hr else ["f1.features:err.flow.facet", "pos.tp:err.flow.facet"]
                assert [p["at"] for p in web] == want, web
                names = {"dig": "tp.dig_lr", "f1": "formula.ms1", "pos": "tp.position"}
                assert all(p["params"]["step"] == names[p["at"].split(".")[0]] and p["params"]["have"] == res for p in web), web
                step_name = names[web[0]["at"].split(".")[0]]
                text = r["problems"][0]["text"]
                assert step_name in text and ("high resolution" if hr else "unit resolution") in text, text
            step(case + ": the page and flow run refuse the same steps on the facets of the files", facets)

            def same(run=run, ids=ids, pl=pl, case=case):
                r = pg.evaluate(RUN, [pl, SAMPLES, PARENT]); assert not r.get("err"), (r.get("err"), r["ev"][-4:])
                kept = pg.evaluate(KEPT); bad = []
                if os.environ.get("NEG_LR_KEEP"):
                    k = Path(os.environ["NEG_LR_KEEP"]) / case.replace("+", "p").replace("-", "n"); shutil.copytree(run / "out", k / "native", dirs_exist_ok=True)
                    (k / "web").mkdir(exist_ok=True)
                    for n, t in kept.items(): (k / "web" / n).write_text(t)
                for i in ids:
                    if not any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]):
                        bad.append((i, "not run in the browser")); continue
                    nat = [x for x in (run / "out").iterdir() if x.name.startswith(i + ".")]
                    web = next((t for k, t in kept.items() if k.startswith(i + ".")), None)
                    if len(nat) != 1 or web is None:
                        bad.append((i, "missing", [x.name for x in nat], sorted(kept))); continue
                    n = nat[0].read_text()
                    if n != web:
                        at = next((j for j, (x, y) in enumerate(zip(n, web)) if x != y), min(len(n), len(web)))
                        bad.append((i, "first difference at", at, n[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80]))
                assert not bad, bad
                ion = "[M-H]-" if pol < 0 else "[M+H]+"
                assert ion in kept[next(k for k in kept if k.startswith("gen."))], "the hypotheses are not " + ion
            step(case + ": every text of the browser is the file of the native flow run", same)
            pg.context.close()
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
