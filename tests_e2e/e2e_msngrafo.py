"""api/msn_grafo on the built site: the .wasm msngrafo (crates/mzlab-msngrafo-wasm, wasm-feat/msngrafo/) answers the graph of a synthetic MSn infusion from the .mzidx and the
mzML in OPFS, with the same bytes as `mzlab-cli msn-graph` (only the SHA-256 of the index may differ), a bad number is a keyed error, the export route gives CSV, MGF or MSP text, api/msnlib/aggiungi adds an entry to the library once,
MZLAB_LR_MISS stays empty and the .wasm is under 512000 bytes. Needs the site (python3 tools/prepara_sito.py); the module is built here when it is not in site/static/wasm-feat."""
import hashlib, json, re, shutil, subprocess, sys
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))

feat = ROOT / "site" / "static" / "wasm-feat"
mod = feat / "msngrafo"
if not (mod / "mzlab_msngrafo_wasm_bg.wasm").exists():
    sh = lambda *a: subprocess.run(a, cwd=ROOT, check=True, capture_output=True)
    sh("cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-msngrafo-wasm")
    mod.mkdir(parents=True, exist_ok=True)
    sh("wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(mod), "target/wasm32-unknown-unknown/release/mzlab_msngrafo_wasm.wasm")
    if shutil.which("wasm-opt"):
        bg = str(mod / "mzlab_msngrafo_wasm_bg.wasm")
        sh("wasm-opt", "-Oz", "--enable-bulk-memory", "--enable-sign-ext", "--enable-mutable-globals", "--enable-nontrapping-float-to-int", "-o", bg, bg)
    mp = feat / "manifest.json"
    m = json.loads(mp.read_text()) if mp.exists() else {}
    m["msngrafo"] = {"js": "msngrafo/mzlab_msngrafo_wasm.js", "wasm": "msngrafo/mzlab_msngrafo_wasm_bg.wasm", "sha256": hashlib.sha256((mod / "mzlab_msngrafo_wasm_bg.wasm").read_bytes()).hexdigest()}
    mp.write_text(json.dumps(m, indent=1))
for f in ("rust-msngrafo.js", "rust-msngrafo-pagina.js", "rust-worker.js", "rust-bridge.js", "browser.js"):
    shutil.copy(ROOT / "mzlab" / "web" / f, ROOT / "site" / "static" / f)

D = synth("msngrafo")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
ION = "C8H11N4O2+"
cli = subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(F), "--formula", ION], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout.rstrip("\n")
mask = lambda t: re.sub(r'"sha256_idx":"[0-9a-f]*"', '"sha256_idx":""', t)
r = Run(port=8978, wd="/tmp/wd_msngrafo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [F], 6000, 1)
        got = {}
        def same():
            got["t"] = pg.evaluate("fetch('api/msn_grafo?k=0&formula=C8H11N4O2%2B').then(r=>r.text())")
            assert got["t"].startswith('{"formato":"msngrafo/1"'), got["t"][:200]
            assert mask(got["t"]) == mask(cli), "the site and the command line differ"
        step("the graph of the site is the one of the command line", same)
        def cuts():
            smi, q = "CNC(=O)c1cnc(N)nc1C", "k=0&formula=C8H11N4O2%2B&smiles=CNC%28%3DO%29c1cnc%28N%29nc1C&strutture=n3"
            want = subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(F), "--formula", ION, "--smiles", smi, "--strutture", "n3"], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout.rstrip("\n")
            t = pg.evaluate(f"fetch('api/msn_grafo?{q}').then(r=>r.text())")
            assert mask(t) == mask(want) and '"strutture":{' in t, "the site and the command line differ"
            assert '"strutture"' not in pg.evaluate("fetch('api/msn_grafo?k=0&formula=C8H11N4O2%2B&smiles=CCO').then(r=>r.text())")
            res = pg.evaluate("fetch('api/msn_grafo?k=0&strutture=n3').then(async r=>[r.status, await r.json()])")
            assert res[0] == 400 and res[1]["error_key"] == "err.msngrafo.noSmiles", res
        step("the cleavages of a node (strutture=) are the ones of the command line; without the key the graph is unchanged", cuts)
        def masses():
            j = json.loads(pg.evaluate("fetch('api/msn_grafo?k=0').then(r=>r.text())"))
            assert j["ione"] is None and len(j["nodi"]) > 3, (j["ione"], len(j["nodi"]))
        step("without a formula the graph is by masses", masses)
        def bad():
            res = pg.evaluate("fetch('api/msn_grafo?k=0&ppm_formula=abc').then(async r=>[r.status, await r.json()])")
            assert res[0] == 400 and res[1]["error_key"] == "err.request.badNumber", res
        step("a bad number is a keyed error", bad)
        def export():
            res = pg.evaluate("fetch('api/msn_grafo/export?k=0&formato=msp&pulito=1').then(async r=>[r.status, r.headers.get('content-type'), await r.text(), r.headers.get('content-disposition')])")
            assert res[0] == 200 and res[1] == "text/plain" and "Num Peaks:" in res[2] and res[3] == 'attachment; filename="msn-graph.msp"', res[:2] + res[3:]
            csv = pg.evaluate("fetch('api/msn_grafo/export?k=0&formato=csv').then(r=>r.text())")
            assert csv.startswith("albero,livello,"), csv[:80]
            bad = pg.evaluate("fetch('api/msn_grafo/export?k=0&formato=xls').then(r=>r.status)")
            assert bad == 400, bad
        step("the export gives CSV and MSP text and a bad format is a keyed error", export)
        def library():
            post = "(b)=>fetch('api/msnlib/aggiungi',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(b)}).then(async r=>[r.status,await r.json()])"
            body = {"libreria": "", "molecola_id": "STD-1", "k": 0, "formula": "C8H11N4O2+", "tol": "nom", "ce": 1, "scelte": []}
            st, a = pg.evaluate(post, body)
            assert st == 200 and a["doppione"] is False and a["libreria"].startswith('{"formato":"msnlib/1"'), (st, str(a)[:200])
            st, again = pg.evaluate(post, {**body, "libreria": a["libreria"]})
            assert st == 200 and again["doppione"] is True and again["libreria"] == a["libreria"], (st, str(again)[:200])
            st, err = pg.evaluate(post, {"libreria": "", "molecola_id": "STD-1", "k": 0})
            assert st == 400 and err["error_key"] == "err.msnlib.noIon", (st, err)
        step("the library gets an entry once, the same one is a duplicate and a graph without ion is a keyed error", library)
        def search():
            post = "(u,b)=>fetch(u,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(b)}).then(async r=>[r.status,await r.json()])"
            body = {"libreria": "", "molecola_id": "STD-1", "k": 0, "formula": "C8H11N4O2+", "tol": "nom", "ce": 1, "scelte": []}
            st, a = pg.evaluate(post, ["api/msnlib/aggiungi", body])
            assert st == 200, (st, a)
            q = {k: v for k, v in body.items() if k != "molecola_id"}
            st, c = pg.evaluate(post, ["api/msnlib/cerca", {**q, "libreria": a["libreria"]}])
            assert st == 200 and c["formato"] == "msncerca/1" and c["candidati"][0]["id"] == "STD-1", (st, str(c)[:300])
            assert c["candidati"][0]["analogo"] is None and abs(c["candidati"][0]["delta_ppm"]) < 1, c["candidati"][0]
            st, e = pg.evaluate(post, ["api/msnlib/cerca", {**q, "libreria": a["libreria"], "top": "abc"}])
            assert st == 400 and e["error_key"] == "err.request.badNumber", (st, e)
            st, e = pg.evaluate(post, ["api/msnlib/cerca", {**q, "libreria": "no"}])
            assert st == 400 and e["error_key"] == "err.msnlib.bad", (st, e)
        step("the library search finds the entry of the same infusion and bad options or a bad library are keyed errors", search)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).slice()")
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(miss)) if miss else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
