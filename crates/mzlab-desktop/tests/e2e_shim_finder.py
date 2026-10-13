"""mzFinder in the desktop shim, in Chromium, without Tauri: the page of the desktop (dist/, made by prepare.py: no Pyodide, no wasm-feat/), mzFinder on, against the real native
engine (`cargo run --example serve`, the mzlab:// protocol over HTTP). A synthetic high-resolution series chosen with «Choose mzML...» (the bytes go to the app once): classify, the Dig,
its details and the table of the page. The Dig equals the one of `cargo run --example finder` (timing aside) and no request names pyodide or wasm-feat. Run from the repository root:
  python3 tools/build_site.py --no-pyodide && python3 crates/mzlab-desktop/prepare.py && python3 crates/mzlab-desktop/tests/e2e_shim_finder.py"""
import json, os, re, shutil, subprocess, sys, tempfile, threading, urllib.request
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tests_e2e"))
import lib  # noqa: F401,E402  (its wait_for_function polls: the page CSP forbids eval)
DIST = ROOT / "crates" / "mzlab-desktop" / "dist"
PAGE_PORT, ENGINE_PORT = 8851, 8852
PARENT = {"name": "parent", "neutral": "C21H32N2O2", "adduct": "[M+H]+"}
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + (str(e) or repr(e))[:900]))

if not (DIST / "index.html").exists():
    print("  ", ("dist", "FAIL run tools/build_site.py --no-pyodide and crates/mzlab-desktop/prepare.py first")); sys.exit(1)
try:
    import numpy
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="shim_finder_"))
ser, nat = tmp / "ser", tmp / "nat"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
idx = sorted(ser.glob("*.mzidx"))
files = [f.with_suffix(".mzML") for f in idx]
nat.mkdir()
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-feat-wasm", "--example", "finder", "--", str(nat), *map(str, idx), "--parent", PARENT["neutral"],
                "--rules", str(ROOT / "TP_Mine" / "data" / "regole_tp.json")], cwd=ROOT, check=True, capture_output=True)
native = {"run": json.loads((nat / "run.json").read_text()), "detail": json.loads((nat / "detail.json").read_text())}
# the MSn file of the standard (a structure with the formula of the parent) and the tree of the Python engine (tools/finder_riferimento.py) it is compared with
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools"))
import finder_riferimento as FR  # noqa: E402
from types import SimpleNamespace  # noqa: E402
SMI, ION = "C1CCC(CC1)N(CC)C(=O)c1ccc(cc1)OCCN1CCCC1", "C21H33N2O2"
(tmp / "msn").mkdir()
msn_ml = FR.MSY.write_msn(tmp / "msn" / "std_msn.mzML", FR.msn_nodes(numpy.random.default_rng(7), ION, SMI), n_scans=5)
r = FR.Run(str(msn_ml)); TREE = FR.MS.build_tree(r, ION, SMI); r.close()
EX = SimpleNamespace(tree=TREE, smiles=SMI)
def ties(a, b, at):
    """top_sites: the order of sites whose log-likelihoods differ by rounding only (the sums of numpy) is free; the last group may be cut anywhere"""
    ll = lambda x: next(v for v in x.values() if isinstance(v, float))
    near = lambda u, v: abs(u - v) <= 1e-9 * max(1.0, abs(u), abs(v))
    assert len(a) == len(b), (at, len(a), len(b))
    i = 0
    while i < len(a):
        j = i
        while j < len(a) and near(ll(a[j]), ll(a[i])): j += 1
        for x, y in zip(a[i:j], b[i:j]): assert near(ll(x), ll(y)), (at, x, y)
        if j < len(a):
            key = lambda z: z["site"]
            close(sorted(a[i:j], key=key), sorted(b[i:j], key=key), f"{at}[{i}:{j}]")
        i = j


def close(a, b, at=""):
    """the same JSON, numbers to 1e-9 (relative)"""
    if isinstance(a, dict) and isinstance(b, dict):
        assert set(a) == set(b), (at, sorted(set(a) ^ set(b)))
        for k in a:
            if k == "top_sites" and isinstance(a[k], list): ties(a[k], b[k], f"{at}.{k}")
            else: close(a[k], b[k], f"{at}.{k}")
    elif isinstance(a, list) and isinstance(b, list):
        assert len(a) == len(b), (at, len(a), len(b))
        for i, (x, y) in enumerate(zip(a, b)): close(x, y, f"{at}[{i}]")
    elif isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        assert abs(a - b) <= 1e-9 * max(1.0, abs(a), abs(b)), (at, a, b)
    else:
        assert a == b, (at, a, b)


def py_localization(d, library):
    """what the Python engine puts in detail.localization for the candidate of detail `d` (its ion = the neutral formula + H)"""
    els = TREE.els
    v = FR.FO.vec(d["row"]["formula"], els); v[els.index("H")] += 1
    m2 = d["ms2"]["candidate"]
    r = FR.LZ.localize(FR.FO.fmt(v, els), ION, m2["mz"], m2["rel"], library, FR.FO.FormulaSpace(v, els), groups=None, tp_mz=d["row"]["mz"])
    r["n_atoms"] = TREE.subs.mol.n
    if r.get("ok"):
        r["n_groups"] = sum(1 for g in FR.LZ.default_groups(TREE.subs.mol).values() if g & set(r["region_atoms"]))
    return FR._plain(r)

# the low-resolution series (TP_Mine/tests/lr_synth.py) and its Dig by the native example of the .wasm (the same pipeline)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402
lrs = lr_synth.write_series(tmp / "lr")
LR_FILES = [{"name": f["name"], "type": f["type"], "time": f["time"], "kind": "full"} for f in lrs]
LR_PARENT = {"name": "parent", "neutral": "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O", "adduct": "[M+H]+"}
(tmp / "lr_files.json").write_text(json.dumps(LR_FILES)); (tmp / "lr_parent.json").write_text(json.dumps(LR_PARENT))
(tmp / "lr_nat").mkdir()
subprocess.run(["cargo", "run", "--release", "-q", "-p", "mzlab-finder-lr-wasm", "--example", "finder_lr", "--", str(tmp / "lr_nat"), str(tmp / "lr_files.json"), str(tmp / "lr_parent.json"),
                str(ROOT / "TP_Mine" / "data" / "regole_tp.json"), *[f["path"] for f in lrs]], cwd=ROOT, check=True, capture_output=True)
lr_native = {"run": json.loads((tmp / "lr_nat" / "run.json").read_text()), "detail": json.loads((tmp / "lr_nat" / "detail.json").read_text())}
subprocess.run(["cargo", "build", "-q", "-p", "mzlab-desktop", "--example", "serve"], cwd=ROOT, check=True)
eng = subprocess.Popen([str(ROOT / "target" / "debug" / "examples" / "serve"), str(ENGINE_PORT)], stdout=subprocess.PIPE, text=True,
                       env={**os.environ, "MZLAB_LIBDIR": str(tmp / "Librerie"), "MZLAB_DATA": str(tmp / "app")})
for line in eng.stdout:
    if line.startswith("READY"): break

class Quiet(SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
page = ThreadingHTTPServer(("127.0.0.1", PAGE_PORT), partial(Quiet, directory=str(DIST)))
threading.Thread(target=page.serve_forever, daemon=True).start()

FAKE_TAURI = """window.__TAURI__ = { core: { invoke: async () => [] }, event: { listen: async () => () => {} }, window: { getCurrentWindow: () => ({ setTitle: () => {} }) } };"""
def native_route(route):                 # mzlab.localhost -> the engine over HTTP (what the window does with the custom protocol)
    u = route.request.url.replace("http://mzlab.localhost", f"http://127.0.0.1:{ENGINE_PORT}")
    try:
        data = route.request.post_data_buffer if route.request.method == "POST" else None
        with urllib.request.urlopen(urllib.request.Request(u, data=data, method=route.request.method)) as r: status, body, ctype = r.status, r.read(), r.headers["Content-Type"]
    except urllib.error.HTTPError as e: status, body, ctype = e.code, e.read(), e.headers["Content-Type"]
    route.fulfill(status=status, body=body, headers={"Content-Type": ctype, "Access-Control-Allow-Origin": "*"})

errs, reqs = [], []
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        ctx = b.new_context(user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36", viewport={"width": 1280, "height": 900}, reduced_motion="reduce")
        ctx.add_init_script(FAKE_TAURI)
        ctx.add_init_script("try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        ctx.route(re.compile(r"http://mzlab\.localhost/.*"), native_route)
        ctx.on("request", lambda r: reqs.append(r.url))
        pg = ctx.new_page()
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.goto(f"http://127.0.0.1:{PAGE_PORT}/index.html"); pg.wait_for_selector("#drop", timeout=300000)
        pg.wait_for_function("typeof window.TPCALL === 'function' && window.MZLAB_DESKTOP === true", timeout=300000)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        got = {}
        def choose():
            pg.set_input_files("#tp-in", [str(f) for f in files])
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= %d" % len(files), timeout=300000)
            m = pg.inner_text("#tp-msg"); assert "error" not in m.lower(), m
            info = pg.evaluate("([n]) => window.TPCALL('classify', n)", [[f.name for f in files]])
            assert [i["name"] for i in info] == [f.name for f in files] and all(i.get("hr_kind") == "hr" for i in info), info
            got["files"] = [{"name": i["name"], "type": i["type"], "time": i.get("time"), "kind": "hr"} for i in info]
        step("Choose mzML: the files go to the app, classify says high resolution", choose)
        def dig():
            run = pg.evaluate("([f, p]) => window.TPCALL('run', f, p, {}, '')", [got["files"], PARENT])
            a, n = dict(run), dict(native["run"])
            for k in ("timing", "settings"): a.pop(k, None); n.pop(k, None)
            assert a == n, [k for k in n if a.get(k) != n.get(k)]
            assert run["n_candidates"] >= 1 and set(run["timing"]) >= {"feature", "align", "proofs", "dig", "total"}, run["timing"]
        step("run: the Dig equals the native example's (timing aside)", dig)
        def details():
            for d in native["detail"]: assert pg.evaluate("([i]) => window.TPCALL('detail', i)", [d["id"]]) == d, d["id"]
        step("detail of the first candidates equals the native example's", details)
        def ui():
            pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            pg.locator("#hr-t tr.clk").first.click(); pg.wait_for_timeout(1500)
            m = pg.inner_text("#tp-msg"); assert "error" not in m.lower(), m
        step("the page: Dig, table and detail of a candidate", ui)
        def transformations():
            user = "my rule;+O;a row of the user"
            run = pg.evaluate("([f, p, t]) => window.TPCALL('run', f, p, {}, t)", [got["files"], PARENT, user])
            assert run["n_candidates"] >= 1, run["n_candidates"]
            r = pg.evaluate("([f, p]) => window.TPCALL('run', f, p, {}, 'no change here').then(() => 'ran', e => String(e.message || e))", [got["files"], PARENT])
            assert "invalid line" in r, r
            rows = pg.evaluate("() => window.TPCALL('transf_rows', 'my rule;+O;a row\\nbad line')")
            rows = json.loads(rows) if isinstance(rows, str) else rows
            assert len(rows["rows"]) >= 1, rows
        step("a list of transformations of the user runs; a bad row says what is wrong", transformations)
        def msn_standard():
            pg.set_input_files("#tp-in", [str(msn_ml)])
            pg.wait_for_function("[...document.querySelectorAll('#tp-files tr')].some(r => r.textContent.trim().startsWith('std_msn'))", timeout=300000)
            info = pg.evaluate("([n]) => window.TPCALL('classify', n)", [["std_msn.mzML"]])
            assert info[0].get("hr_kind") == "msn", info
            files = got["files"] + [{"name": "std_msn.mzML", "type": info[0]["type"], "time": None, "kind": "msn"}]
            call = lambda fn, *a: pg.evaluate("([fn, a]) => window.TPCALL(fn, ...a)", [fn, list(a)])
            run = call("run", files, {"name": "parent", "smiles": SMI}, {}, "")
            assert [x["name"] for x in run["files"]] == [x["name"] for x in got["files"]], run["files"]
            close(run["tree"], FR._plain(FR.ExperimentHR._tree_summary(EX)), "tree")
            close(run["calibration"], FR._plain(TREE.cal.as_dict()), "calibration")
            close(call("msn_tree"), run["tree"], "msn_tree")
            close(call("frag_tree"), FR._plain(FR.FT._hr_msn(EX)), "frag_tree")
            ds = [call("detail", x["id"]) for x in run["rows"][:12]]
            pm = next((d["ms2"]["parent"] for d in ds if d.get("ms2")), None)     # the synthetic series has no MS2 of the parent in the Dig: then none is localised
            library = pm and FR.iimn.build_library(TREE, dda_peaks=(numpy.array(pm["mz"]), numpy.array(pm["rel"])))
            rows = {x["id"]: x for x in run["rows"]}
            for d in ds:
                if not (d.get("ms2") and d["row"]["formula"]):
                    assert d["localization"] is None and rows[d["id"]]["region"] is None, d["id"]
                    close(call("frag_tree_compare", d["id"]), FR._plain(FR.FT.compare_hr(FR.FT._hr_msn(EX), None)), f"compare {d['id']}")
                    continue
                want = py_localization(d, library)
                close(d["localization"], want, f"localization {d['id']}")
                close(d["ms2"]["evidence"], want.get("evidence", []), f"evidence {d['id']}")
                ok = want.get("ok")
                assert rows[d["id"]]["region"] == (want["region"] if ok else None) and rows[d["id"]]["margin"] == (round(want["margin"], 2) if ok else None), ("row", d["id"])
                close(call("frag_tree_compare", d["id"]), FR._plain(FR.FT.compare_hr(FR.FT._hr_msn(EX), want)), f"compare {d['id']}")
        step("run with an MSn file: tree, calibration, localization, frag_tree and frag_tree_compare (= the Python)", msn_standard)
        def lr_series():
            pg.set_input_files("#tp-in", [f["path"] for f in lrs])
            pg.wait_for_function("[...document.querySelectorAll('#tp-files tr')].filter(r => r.textContent.trim().startsWith('lr_')).length >= %d" % len(lrs), timeout=300000)
            run = pg.evaluate("([f, p]) => window.TPCALL('run', f, p, {}, '')", [LR_FILES, LR_PARENT])
            a, n = dict(run), dict(lr_native["run"])
            a.pop("timing", None); n.pop("timing", None)
            assert a == n, [k for k in n if a.get(k) != n.get(k)]
            assert run["rows"][0]["label"] == "progenitore", run["rows"][0]
            assert lr_native["detail"], "no row passes the criteria"
            for d in lr_native["detail"]: assert pg.evaluate("([i]) => window.TPCALL('detail', i)", [d["id"]]) == d, d["id"]
            t = pg.evaluate("() => window.TPCALL('frag_tree')")
            assert t == {"ok": False, "mode": "lr", "note": "no MS2 of the parent in the files", "nodes": []}, t
        step("a low-resolution series: the Dig and the details equal the native example's", lr_series)
        def clean():
            bad = [u for u in reqs if re.search(r"pyodide|wasm-feat|mzlab\.zip", u, re.I)]
            assert not bad, bad[:5]
        step("no request names pyodide, wasm-feat or mzlab.zip", clean)
        b.close()
finally:
    eng.terminate(); page.shutdown()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
