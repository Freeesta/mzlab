"""mzFinder in Rust (TP_Mine/files/tpmine-worker.js): with the .mzidx of a high-resolution series in OPFS, classify, the Dig (run), detail, frag_tree, frag_tree_compare,
formula_info, identity_graph and polarity_pairs are answered by the `finder` .wasm and the pool of feat-worker.js, and Pyodide is never downloaded.
The JSON of the Dig and the details equal the ones of the native build (`cargo run --example finder`) on the same files.
Builds the site (python3 tools/build_site.py) and static/wasm-feat (tools/build_feat_wasm.sh); SKIP without cargo, wasm-bindgen or numpy. MZLAB_PIPE_SITE=<built site with
static/wasm-feat> reuses a site (the TP_Mine files are copied over it). Run from the repo root: python3 tests_e2e/e2e_tpmine_rust.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="tpmine_rust_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")), capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
ser, nat = tmp / "ser", tmp / "nat"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
idx = sorted(ser.glob("*.mzidx"))
data = site / "static" / "dati_finder"; data.mkdir(exist_ok=True)
for f in idx:
    shutil.copy2(f, data / (f.stem + ".mzML.mzidx"))      # the name the Data tab gives the index of «x.mzML»
    shutil.copy2(f.with_suffix(".mzML"), data / (f.stem + ".mzML"))
names = [f.stem + ".mzML" for f in idx]
PARENT = {"name": "parent", "neutral": "C21H32N2O2", "adduct": "[M+H]+"}
nat.mkdir()
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "finder", "--", str(nat), *map(str, idx), "--parent", PARENT["neutral"],
                "--rules", str(ROOT / "TP_Mine" / "data" / "regole_tp.json")], cwd=ROOT, check=True, capture_output=True)
native = {"run": json.loads((nat / "run.json").read_text()), "detail": json.loads((nat / "detail.json").read_text())}

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_finder/" + n + ".mzidx")).body.pipeTo(w); }
}"""
PUTF = PUT.replace('"idx"]', '"files"]').replace('n + ".mzidx", { create', 'n, { create').replace('n + ".mzidx")).body', 'n)).body')
IDENT = {"features": [{"k": "F1", "id": "1", "mz": 301.1, "rt": 5.0, "a": [[0, 1e5], [1, 2e5], [2, 3e5]]}, {"k": "F2", "id": "2", "mz": 323.082, "rt": 5.01, "a": [[0, 2e5], [1, 4e5], [2, 6e5]]}],
         "samples": 3, "pol": 1, "edges": []}
POL = {"pos": [{"k": "p1", "mz": 301.1, "rt": 5.0}], "neg": [{"k": "n1", "mz": 299.0854, "rt": 5.02}], "ppm": 5, "rt_min": 0.15}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
        pg.evaluate(PUT, names)
        call = lambda fn, *a: pg.evaluate("([fn, a]) => window.TPCALL(fn, ...a)", [fn, list(a)])
        got = {}

        def classify():
            info = call("classify", names)
            assert [i["name"] for i in info] == names and all(i.get("hr_kind") == "hr" for i in info), info
            got["files"] = [{"name": i["name"], "type": i["type"], "time": i.get("time"), "kind": "hr"} for i in info]
        step("classify: the .mzidx of OPFS, high resolution", classify)

        def dig():
            run = call("run", got["files"], PARENT, {}, "")
            a, n = dict(run), dict(native["run"])
            for k in ("timing", "settings"): a.pop(k, None); n.pop(k, None)
            assert a == n, [k for k in n if a.get(k) != n.get(k)]
            assert run["n_candidates"] >= 1 and set(run["timing"]) >= {"feature", "align", "proofs", "dig", "total"}, run["timing"]
            got["run"] = run
        step("run: the Dig equals the native one (timing aside)", dig)

        def details():
            for d in native["detail"]: assert call("detail", d["id"]) == d, d["id"]
        step("detail of the first candidates equals the native one", details)

        def tree():
            t = call("frag_tree")
            assert t["mode"] == "hr" and (t["ok"] or "no MS2 of the parent" in t["note"]), t
            c = call("frag_tree_compare", native["detail"][0]["id"])
            assert c == {"ok": False, "note": "no fragment of the product with a known relation to the parent", "marks": {}}, c
        step("frag_tree and frag_tree_compare of the Rust Dig", tree)

        def stateless():
            f = call("formula_info", "C21H32N2O2", "[M+H]+")
            assert abs(f["mz"] - 345.2537) < 1e-3, f
            g = call("identity_graph", IDENT); assert "identities" in g and "edges" in g, g
            q = call("polarity_pairs", POL); assert len(q) == 1 and q[0]["pos"] == "p1" and q[0]["neg"] == "n1", q
        step("formula_info, identity_graph, polarity_pairs", stateless)

        def ui():
            pg.evaluate(PUTF, names)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.click("#tp-page"); pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 7", timeout=120000)
            pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            assert pg.locator("#hr-t tr.clk").count() >= 1
            pg.locator("#hr-t tr.clk").first.click(); pg.wait_for_timeout(1500)
            m = pg.inner_text("#tp-msg"); assert "error" not in m.lower(), m
        step("the page: Reload from Data, Dig, table and detail of a candidate", ui)

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
            assert [u for u in reqs if "wasm-feat/finder" in u], "the finder .wasm was fetched"
        step("no pyodide* request with high-resolution input", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
