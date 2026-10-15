"""The low-resolution Dig of mzFinder in Rust (TP_Mine/files/finder-lr.js, crates/mzlab-finder-lr-wasm): with the .mzidx of a unit-resolution full-scan series
(TP_Mine/tests/lr_synth.py) in OPFS, run, detail, frag_tree and frag_tree_compare are answered by the `finder_lr` .wasm and the pool of feat-worker.js, and
Pyodide is never downloaded. The JSON of the Dig and the details equal the ones of the native build (`cargo run --example finder_lr`) on the same files.
Builds the site (python3 tools/build_site.py) and static/wasm-feat (tools/build_feat_wasm.sh); SKIP without cargo, wasm-bindgen or numpy. MZLAB_PIPE_SITE=<built site with
static/wasm-feat> reuses a site (the TP_Mine files are copied over it). Run from the repo root: python3 tests_e2e/e2e_tpmine_lr_rust.py"""
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
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-finder-lr-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402

tmp = Path(tempfile.mkdtemp(prefix="tpmine_lr_rust_"))
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
series = lr_synth.write_series(ser)
FILES = [{"name": f["name"], "type": f["type"], "time": f["time"], "kind": "full"} for f in series]
names = [f["name"] for f in FILES]
PARENT = {"name": "parent", "neutral": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O", "adduct": "[M+H]+"}
nat.mkdir()
(tmp / "files.json").write_text(json.dumps(FILES)); (tmp / "parent.json").write_text(json.dumps(PARENT))
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-finder-lr-wasm", "--example", "finder_lr", "--", str(nat), str(tmp / "files.json"), str(tmp / "parent.json"),
                str(ROOT / "TP_Mine" / "data" / "regole_tp.json"), *[f["path"] for f in series]], cwd=ROOT, check=True, capture_output=True)
native = {"run": json.loads((nat / "run.json").read_text()), "detail": json.loads((nat / "detail.json").read_text())}
data = site / "static" / "dati_finder"; data.mkdir(exist_ok=True)
for n in names:
    shutil.copy2(nat / (n + ".mzidx"), data / (n + ".mzidx"))

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_finder/" + n + ".mzidx")).body.pipeTo(w); }
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
        pg.evaluate(PUT, names)
        call = lambda fn, *a: pg.evaluate("([fn, a]) => window.TPCALL(fn, ...a)", [fn, list(a)])
        got = {}

        def dig():
            run = call("run", FILES, PARENT, {}, "")
            a, n = dict(run), dict(native["run"])
            for k in ("timing",): a.pop(k, None); n.pop(k, None)
            assert a == n, [k for k in n if a.get(k) != n.get(k)]
            assert run["rows"][0]["label"] == "progenitore" and set(run["timing"]) >= {"feature", "candidates", "align", "dig", "total"}, run["timing"]
            assert all(r["isf"] is None for r in run["rows"]), "isf"
            got["run"] = run
        step("run: the Dig equals the native one (timing aside)", dig)

        def details():
            assert native["detail"], "no row passes the criteria"
            for d in native["detail"]: assert call("detail", d["id"]) == d, d["id"]
        step("detail of the parent and of every row that passes equals the native one", details)

        def tree():
            t = call("frag_tree")
            assert t == {"ok": False, "mode": "lr", "note": "no MS2 of the parent in the files", "nodes": []}, t
            c = call("frag_tree_compare", native["detail"][-1]["id"])
            assert c == {"ok": False, "note": "no MS2 of the product", "marks": {}}, c
        step("frag_tree and frag_tree_compare of the Rust Dig (no MS2 in the series)", tree)

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
            assert [u for u in reqs if "wasm-feat/finder-lr" in u], "the finder-lr .wasm was fetched"
        step("no pyodide* request with a low-resolution full-scan series", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
