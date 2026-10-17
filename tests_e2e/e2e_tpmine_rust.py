"""mzFinder in Rust (TP_Mine/files/tpmine-worker.js): with the .mzidx of a high-resolution series in OPFS, classify, the Dig (run), detail, frag_tree, frag_tree_compare,
formula_info, identity_graph, polarity_pairs and origin_evidence are answered by the `finder` .wasm and the pool of feat-worker.js, and Pyodide is never downloaded.
The JSON of the Dig and the details equal the ones of the native build (`cargo run --example finder`) on the same files. With a synthetic MSn file of the standard
(TP_Mine/tests/msn_synth.py) the `finder_msn` .wasm gives msn_tree, calibration, detail.localization, frag_tree and frag_tree_compare equal to the Python.
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
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
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
# the MSn file of the standard: a structure with the formula of the parent, its tree from the substructures (tools/finder_riferimento.py)
sys.path.insert(0, str(ROOT / "tools")); import finder_riferimento as FR  # noqa: E402
from types import SimpleNamespace  # noqa: E402
from mzlab.reader import mzidx as MZI  # noqa: E402
SMI, ION, MSN = "C1CCC(CC1)N(CC)C(=O)c1ccc(cc1)OCCN1CCCC1", "C21H33N2O2", "std_msn.mzML"
NODES = FR.msn_nodes(numpy.random.default_rng(7), ION, SMI)
msn_ml = FR.MSY.write_msn(data / MSN, NODES, n_scans=5)
r = FR.Run(str(msn_ml)); MZI.build(r, data / (MSN + ".mzidx")); r.close()
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


def py_localization(d, lib):
    """what the Python engine puts in detail.localization for the candidate of detail `d` (its ion = the neutral formula + H)"""
    els = TREE.els
    v = FR.FO.vec(d["row"]["formula"], els); v[els.index("H")] += 1
    m2 = d["ms2"]["candidate"]
    r = FR.LZ.localize(FR.FO.fmt(v, els), ION, m2["mz"], m2["rel"], lib, FR.FO.FormulaSpace(v, els), groups=None, tp_mz=d["row"]["mz"])
    r["n_atoms"] = TREE.subs.mol.n
    if r.get("ok"):
        r["n_groups"] = sum(1 for g in FR.LZ.default_groups(TREE.subs.mol).values() if g & set(r["region_atoms"]))
    return FR._plain(r)

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_finder/" + n + ".mzidx")).body.pipeTo(w); }
}"""
PUTF = PUT.replace('"idx"]', '"files"]').replace('n + ".mzidx", { create', 'n, { create').replace('n + ".mzidx")).body', 'n)).body')
MODULE = """async ([url, idx, parent, dda, cases]) => {
  const m = await import(url + "mzlab_feat_wasm.js");
  await m.default({ module_or_path: url + "mzlab_feat_wasm_bg.wasm" });
  const buf = new Uint8Array(await (await fetch(idx)).arrayBuffer());
  const f = m.finder_msn((k, off, len) => buf.slice(off, off + len), buf.length, parent);
  f.set_parent_ms2(Float64Array.from(dda[0]), Float64Array.from(dda[1]));
  const loc = cases.map((c, i) => ({ l: JSON.parse(f.localize(i, c.neutral, Float64Array.from(c.mz), Float64Array.from(c.rel), c.tp_mz)),
                                    cmp: JSON.parse(f.frag_tree_compare(i)) }));
  return { tree: JSON.parse(f.msn_tree()), loc };
}"""
IDENT = {"features": [{"k": "F1", "id": "1", "mz": 301.1, "rt": 5.0, "a": [[0, 1e5], [1, 2e5], [2, 3e5]]}, {"k": "F2", "id": "2", "mz": 323.082, "rt": 5.01, "a": [[0, 2e5], [1, 4e5], [2, 6e5]]}],
         "samples": 3, "pol": 1, "edges": []}
ORIGIN = [c for c in json.loads((ROOT / "tests" / "golden" / "feat" / "finder.json").read_text())["origin"] if c["out"]][:8]
POL = {"pos": [{"k": "p1", "mz": 301.1, "rt": 5.0}], "neg": [{"k": "n1", "mz": 299.0854, "rt": 5.02}], "ppm": 5, "rt_min": 0.15}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
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

        def msn():
            pg.evaluate(PUT, [MSN])
            info = call("classify", [MSN])
            assert info[0].get("hr_kind") == "msn", info
            files = got["files"] + [{"name": MSN, "type": info[0]["type"], "time": None, "kind": "msn"}]
            run = call("run", files, {"name": "parent", "smiles": SMI}, {}, "")
            assert [x["name"] for x in run["files"]] == [x["name"] for x in got["files"]], run["files"]
            close(run["tree"], FR._plain(FR.ExperimentHR._tree_summary(EX)), "tree")
            close(run["calibration"], FR._plain(TREE.cal.as_dict()), "calibration")
            close(call("msn_tree"), run["tree"], "msn_tree")
            close(call("frag_tree"), FR._plain(FR.FT._hr_msn(EX)), "frag_tree")
            ds = [call("detail", x["id"]) for x in run["rows"][:12]]
            pm = next((d["ms2"]["parent"] for d in ds if d.get("ms2")), None)     # the synthetic series has no MS2 of the parent in the Dig: then none is localised
            lib = pm and FR.iimn.build_library(TREE, dda_peaks=(numpy.array(pm["mz"]), numpy.array(pm["rel"])))
            rows = {x["id"]: x for x in run["rows"]}
            for d in ds:
                if not (d.get("ms2") and d["row"]["formula"]):
                    assert d["localization"] is None and rows[d["id"]]["region"] is None, d["id"]
                    close(call("frag_tree_compare", d["id"]), FR._plain(FR.FT.compare_hr(FR.FT._hr_msn(EX), None)), f"compare {d['id']}")
                    continue
                want = py_localization(d, lib)
                close(d["localization"], want, f"localization {d['id']}")
                close(d["ms2"]["evidence"], want.get("evidence", []), f"evidence {d['id']}")
                ok = want.get("ok")
                assert rows[d["id"]]["region"] == (want["region"] if ok else None) and rows[d["id"]]["margin"] == (round(want["margin"], 2) if ok else None), ("row", d["id"], rows[d["id"]]["region"], rows[d["id"]]["margin"], want.get("region"), want.get("margin"))
                close(call("frag_tree_compare", d["id"]), FR._plain(FR.FT.compare_hr(FR.FT._hr_msn(EX), want)), f"compare {d['id']}")
        step("run with an MSn file: tree, calibration, localization, frag_tree and frag_tree_compare (finder_msn = the Python)", msn)

        def module():
            rng = numpy.random.default_rng(11)
            dda = FR.dda_of(rng, NODES)
            lib = FR.iimn.build_library(TREE, dda_peaks=(numpy.array(dda[0]), numpy.array(dda[1])))
            els, cases, want = TREE.els, [], []
            for ch in FR.CHANGES:
                tp, mz, rel = FR.product_ms2(rng, TREE, lib, FR.change_vec(ch, els))
                neutral = tp.copy(); neutral[els.index("H")] -= 1
                tp_mz = float(FR.FO.ion_mass(tp, els))
                cases.append({"neutral": FR.FO.fmt(neutral, els), "mz": mz, "rel": rel, "tp_mz": tp_mz})
                d = {"row": {"formula": FR.FO.fmt(neutral, els), "mz": tp_mz}, "ms2": {"candidate": {"mz": mz, "rel": rel}}}
                loc = py_localization(d, lib)
                want.append({"l": loc, "cmp": FR._plain(FR.FT.compare_hr(FR.FT._hr_msn(EX), loc))})
            got_ = pg.evaluate(MODULE, ["/static/wasm-feat/finder-msn/", "static/dati_finder/" + MSN + ".mzidx", json.dumps({"name": "parent", "smiles": SMI}), dda, cases])
            close(got_["tree"], FR._plain(FR.ExperimentHR._tree_summary(EX)), "tree")
            assert any(w["l"].get("ok") for w in want), [w["l"].get("note") for w in want]
            for i, (a, w) in enumerate(zip(got_["loc"], want)): close(a, w, f"case {i}")
        step("finder_msn in the page: localize and frag_tree_compare of synthetic products (= the Python)", module)

        def stateless():
            f = call("formula_info", "C21H32N2O2", "[M+H]+")
            assert abs(f["mz"] - 345.2537) < 1e-3, f
            g = call("identity_graph", IDENT); assert "identities" in g and "edges" in g, g
            q = call("polarity_pairs", POL); assert len(q) == 1 and q[0]["pos"] == "p1" and q[0]["neg"] == "n1", q
            for c in ORIGIN: assert call("origin_evidence", c["payload"]) == c["out"], c["payload"]
        step("formula_info, identity_graph, polarity_pairs, origin_evidence (= the Python of tests/golden/feat/finder.json)", stateless)

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
            assert [u for u in reqs if "wasm-feat/finder/" in u], "the finder .wasm was fetched"
            assert [u for u in reqs if "wasm-feat/finder-msn/" in u], "the finder_msn .wasm was fetched"
        step("no pyodide* request with high-resolution input", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
