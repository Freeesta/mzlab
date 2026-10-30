"""Flow «This browser» screen.suspects (flow/lazy/flow-web-suspects.js, `flow_screen_suspects` of the `table` .wasm): over the synthetic identity series (tools/feat_sintetici.py) and a suspect list made here from the
precursors of its MS2 scans (two true suspects, one with the fragments of its spectrum, a decoy 20 mDa away and an absent mass) the plan detect → align → gap-fill, spectra.ms2 → screen.suspects runs in the browser and writes the screen/1
text that `mzlab-cli flow run` writes of the same plan, byte for byte; a second run takes it from OPFS; the engine had no route it could not serve. Builds the site and the .wasm of mzFinder (needs cargo, wasm-bindgen,
python3 and numpy: SKIP without). Run from the repo root: python3 tests_e2e/e2e_flow_suspects.py"""
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
ONE = "SIN_t10min"; MS2 = "MS2_ONE"   # the file whose MS2 scans the plan reads, under a name of its own: the same input cannot be a raw file and an mzML
PROTON = 1.007276

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

tmp = Path(tempfile.mkdtemp(prefix="flow_suspects_"))
site = tmp / "site"; env = dict(os.environ, RUNNER_TEMP=str(tmp / "rt"))
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
from lib_sito import add_wasm; add_wasm(site)
from lib import build_feat; build_feat(site / "static", env)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_sintetici.py", str(ser), "--solo", "identita"], cwd=ROOT, check=True, capture_output=True)
run = tmp / "run"; run.mkdir()
for n in NAMES: shutil.copy2(ser / "identita" / (n + ".mzML"), run / (n + ".mzML"))
shutil.copy2(ser / "identita" / (ONE + ".mzML"), run / (MS2 + ".mzML"))

def native(plan, name):
    (run / (name + ".json")).write_text(json.dumps(plan)); out = run / ("out_" + name)
    subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--", "flow", "run", str(run / (name + ".json")), "--in", str(run), "--out", str(out)], cwd=ROOT, check=True, capture_output=True)
    return out

# 1. the MS2 of the file as MGF: the precursors make the suspect list
PLAN0 = {"format": "mzflow/2", "inputs": [{"name": MS2 + ".mzML", "sha256": "", "type": "mzml"}], "steps": [{"id": "s", "step": "spectra.ms2@1", "in": {"source": ["file:" + MS2 + ".mzML"]}}]}
blocks = mgf_blocks((native(PLAN0, "plan0") / "s.mgf").read_text())
assert len(blocks) >= 2, "the series has no MS2 in " + ONE
(h0, p0), (h1, _) = blocks[0], blocks[1]
neutral = lambda h: float(h["PEPMASS"].split()[0]) - PROTON
frag = ";".join("%.4f" % m for m, _ in sorted(p0, key=lambda p: -p[1])[:3])
LIST = ("name,inchikey,mass,rt,fragments\n"
        f"Suspect one,AAAAAAAAAAAAAA,{neutral(h0):.5f},{float(h0['RTINSECONDS']) / 60:.3f},{frag}\n"
        f"Suspect two,BBBBBBBBBBBBBB,{neutral(h1):.5f},,\n"
        f"Decoy,CCCCCCCCCCCCCC,{neutral(h0) + 0.02:.5f},,\n"
        f"Absent,DDDDDDDDDDDDDD,{neutral(h0) + 250.0:.5f},,\n")
(run / "list.csv").write_text(LIST)
LISTSHA = hashlib.sha256(LIST.encode()).hexdigest()

# 2. the plan: the base chain → spectra.ms2 → screen.suspects
PLAN = {"format": "mzflow/2", "inputs": [{"name": n + ".mzML", "sha256": "", "type": "raw"} for n in NAMES] + [{"name": MS2 + ".mzML", "sha256": "", "type": "mzml"}, {"name": "list.csv", "sha256": LISTSHA, "type": "library"}],
        "steps": [{"id": "idx", "step": "index.mzidx@1", "in": {"source": ["file:" + n + ".mzML" for n in NAMES]}},
                  {"id": "det", "step": "features.detect@1", "in": {"index": ["idx.index"]}},
                  {"id": "ali", "step": "features.align@1", "in": {"peaks": ["det.peaks"]}},
                  {"id": "gap", "step": "features.gapfill@1", "in": {"table": "ali.table", "index": ["idx.index"]}},
                  {"id": "ms2", "step": "spectra.ms2@1", "in": {"source": ["file:" + MS2 + ".mzML"]}},
                  {"id": "scr", "step": "screen.suspects@1", "in": {"table": "gap.table", "suspects": ["file:list.csv"], "spectra": "ms2.spectra"}, "params": {"ppm": 10}}]}
out = native(PLAN, "plan")
NAT = (out / "scr.json").read_bytes()
nat = json.loads(NAT)
print("native:", len(NAT), "bytes,", len(nat["hits"]), "hits")
assert nat["format"] == "screen/1" and nat["suspect_files"][0]["sha256"] == LISTSHA
assert any(c["name"] == "Suspect one" for h in nat["hits"] for c in h["candidates"]), "the true suspect is not found: " + NAT[:600].decode()
# the decoy may hit another feature of the 930 through an adduct by chance; it must not hit the feature of the true suspect, and the absent one must hit nothing
assert not any(c["name"] == "Absent" for h in nat["hits"] for c in h["candidates"]), "the absent suspect is listed"
assert not any(c["name"] == "Decoy" for h in nat["hits"] if h["adduct"] == "[M+H]+" for c in h["candidates"]), "the decoy is listed as [M+H]+"
data = site / "static" / "dati_flow_suspects"; data.mkdir()
for n in NAMES: shutil.copy2(out / ("idx-" + n + ".mzidx"), data / (n + ".mzidx"))
shutil.copy2(run / (MS2 + ".mzML"), data / (MS2 + ".mzML"))

PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_flow_suspects/" + n + ".mzidx")).body.pipeTo(w); }
}"""
# the mzML the page has "open" for spectra.ms2 (the engine keeps the blob of every file it was given)
HOLD = """async (name) => { const b = await (await fetch("static/dati_flow_suspects/" + name + ".mzML")).blob(); window.MZLAB_RUST.noteUpload(name + ".mzML", b); return b.size; }"""
RUN = """async ([plan, list]) => {
  const hex = b => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  S.plan.inputs = JSON.parse(JSON.stringify(plan.inputs)); S.plan.steps = JSON.parse(JSON.stringify(plan.steps));
  S.samplesJson = S.samplesJson || {}; S.samplesJson['list.csv'] = list;
  // the picker of the page records the SHA-256 of the file in the input, which is what tells the runner the list changed
  S.plan.inputs.find(i => i.name === 'list.csv').sha256 = hex(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(list)));
  document.addEventListener('flow:web-step', on);
  try { await FLOW.runners.web.run(S); return { ev }; }
  catch (e) { return { ev, err: { key: e.key, params: e.params, msg: String(e && e.message || e) } }; }
  finally { document.removeEventListener('flow:web-step', on); }
}"""
KEPT = """async () => {
  const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow');
  return await (await (await d.getFileHandle('scr.json')).getFile()).text();
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
        pg.evaluate(HOLD, MS2); pg.evaluate(PUT, NAMES)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web && document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow", timeout=120000)

        def check(r, expect):
            assert not r.get("err"), (r.get("err"), r["ev"][-4:])
            assert any(e["passo"] == "scr" and e["stato"] == expect and e["via"] == "browser" for e in r["ev"]), (expect, [e for e in r["ev"] if e["passo"] == "scr"][-3:])
        def first():
            r = pg.evaluate(RUN, [PLAN, LIST]); check(r, "done")
            for i in ("det", "ali", "ms2"):   # gap fill runs inside the alignment call of the pool
                assert any(e["passo"] == i and e["stato"] == "done" and e["via"] == "browser" for e in r["ev"]), (i, r["ev"][-6:])
            web = pg.evaluate(KEPT).encode()
            at = next((i for i, (x, y) in enumerate(zip(NAT, web)) if x != y), min(len(NAT), len(web)))
            assert sha(web) == sha(NAT) and len(NAT) > 100, ("first difference at", at, len(NAT), len(web), NAT[max(0, at - 80):at + 80], web[max(0, at - 80):at + 80])
        step("screen.suspects writes the screen/1 text of the native flow run (same SHA-256), every job done in the browser", first)
        def again():
            r = pg.evaluate(RUN, [PLAN, LIST]); check(r, "reused")
        step("a second run takes it from OPFS", again)
        def no_miss():
            miss = pg.evaluate("(window.MZLAB_LR_MISS || []).length"); assert miss == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("the engine had no route it could not serve", no_miss)
        def other_list():
            r = pg.evaluate(RUN, [PLAN, LIST + "Extra,EEEEEEEEEEEEEE,901.12345,,\n"]); check(r, "done")
        step("another list is another result (the hash of the list is in the key)", other_list)
        b.close()
finally:
    srv.terminate(); shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
