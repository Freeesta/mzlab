"""mzFinder in the desktop shim, in Chromium, without Tauri: the page of the desktop (dist/, made by prepare.py: no Pyodide, no wasm-feat/), mzFinder on, against the real native
engine (`cargo run --example serve`, the mzlab:// protocol over HTTP). A synthetic high-resolution series chosen with «Choose files...» (the bytes go to the app once): classify, the Dig,
its details and the table of the page. The Dig equals the one of `cargo run --example finder` (timing aside) and no request names pyodide or wasm-feat. Run from the repository root:
  python3 tools/build_site.py && python3 crates/mzlab-desktop/prepare.py && python3 crates/mzlab-desktop/tests/e2e_shim_finder.py"""
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
    print("  ", ("dist", "FAIL run tools/build_site.py and crates/mzlab-desktop/prepare.py first")); sys.exit(1)
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
# the MSn file of the standard (a structure with the formula of the parent) and the frozen answers of the former Python engine on it
sys.path.insert(0, str(ROOT)); sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(ROOT / "TP_Mine" / "tests"))
import msn_synth  # noqa: E402
G = json.loads((ROOT / "tests" / "golden" / "finder_msn_e2e.json").read_text(encoding="utf-8"))
SMI, ION = G["smiles"], G["ion"]
(tmp / "msn").mkdir()
msn_ml = msn_synth.write_msn(tmp / "msn" / "std_msn.mzML", {tuple(tuple(x) for x in k): [tuple(x) for x in peaks] for k, peaks in G["nodes"]}, n_scans=5)
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


# the low-resolution series (TP_Mine/tests/lr_synth.py) and its Dig by the native example of the .wasm (the same pipeline)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402
lrs = lr_synth.write_series(tmp / "lr")
LR_FILES = [{"name": f["name"], "type": f["type"], "time": f["time"], "kind": "full"} for f in lrs]
demo_mz = lr_synth.demo.PARENT_MZ
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

FAKE_TAURI = """try { if (!sessionStorage.getItem("qqq.run")) sessionStorage.setItem("qqq.run", "1"); } catch (_) {}    // a running app: no cold start
window.__TAURI__ = { core: { invoke: async () => [] }, event: { listen: async () => () => {} }, window: { getCurrentWindow: () => ({ setTitle: () => {} }) } };"""
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
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
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
        def tp_table():
            ids = [d["id"] for d in native["detail"]]
            t = pg.evaluate("([i]) => window.TPCALL('tp_table', i)", [ids])
            assert t and t.get("schema") == "tptab/1" and [x["row"] for x in t["rows"]] == ids, t
            for x in t["rows"]: assert all(x.get(k) is not None for k in ("in_t0", "in_blank", "t0_ratio", "blank_ratio")), x
            assert t == pg.evaluate("([i]) => window.TPCALL('tp_table', i)", [ids]), "the table changes between two calls"
        step("tp_table: the table of the products has the t0 and Blank columns filled", tp_table)
        def ui():
            pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
            pg.locator("#hr-t tr.clk").first.click(); pg.wait_for_timeout(1500)
            m = pg.inner_text("#tp-msg"); assert "error" not in m.lower(), m
        step("the page: Dig, table and detail of a candidate", ui)
        def pipeline_pool():
            r = pg.evaluate("""async ([names, rulesUrl, parent]) => {
                const pool = (await import('/static/mzfinder/feat-desktop-pool.js')).createDesktopPool();
                const list = await pool.listIdx(), files = names.map(n => ({ name: n + '.mzidx', path: 'mzlab/idx/' + n + '.mzidx' }));
                const prev = await pool.preview(files[0], ''), stored = await pool.detectAll(files, '');
                const table = await pool.align(stored, files.map(f => f.path), '');
                const rules = new Uint8Array(await (await fetch(rulesUrl)).arrayBuffer());
                const h = await pool.hypotheses({ rules, parent, parentRt: NaN, paths: files.map(f => f.path), recipe: '', structure: '' });
                const e = await pool.evidence({ minutes: files.map(() => NaN), parentMz: NaN, parentRt: NaN, recipe: '' });
                const sha = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', e.table))).map(x => x.toString(16).padStart(2, '0')).join('');
                const smp = await pool.table('samples', { files: JSON.stringify({ files: names.map(n => ({ name: n, sha256: '' })) }) });
                const qc = JSON.parse(await pool.table('qc', { samples: smp, spec: '', sha256: sha, polarity: 1 }));
                const csv = await pool.table('export', { samples: smp, spec: '', sha256: sha, polarity: 1, export_kind: 'full_mzmine' });
                let wrong = ''; try { await pool.table('qc', { samples: smp, spec: '', sha256: '0'.repeat(64), polarity: 1 }); } catch (x) { wrong = x.key || String(x); }
                return { qc: Array.isArray(qc.file) ? qc.file.length : -1, csv: csv.length, wrong, list, crate: await pool.crateVersion(), prev: Object.keys(prev), n: stored.length, table: table.length, evid: e.table.length, tried: h.tried, reg: !!h.registry };
            }""", [[f.name for f in files], "/static/mzfinder/regole_tp.json", PARENT["neutral"]])
            assert all(f.name + ".mzidx" in r["list"] for f in files), r["list"]
            assert r["n"] == len(files) and r["table"] > 100 and r["evid"] >= r["table"] and r["reg"] and r["crate"], r
            assert r["qc"] == len(files) and r["csv"] > 100 and r["wrong"] == "error.featState", (r["qc"], r["csv"], r["wrong"])
            assert "ms" in r["prev"] and "crate" in r["prev"], r["prev"]
        step("Pipeline panel pool: listIdx, preview, detect, align, hypotheses, proofs and the views of the table on the native routes", pipeline_pool)
        def pipeline_panel():
            pg.evaluate("PIPE.open()"); pg.wait_for_function("PIPE.state.crate", timeout=60000); crate = pg.evaluate("PIPE.state.crate")
            gate = {"format": "cancello/2", "crate_version": crate, "data": "2026-10-12", "vista_candidati": True, "vista_tabella": True, "concordanza_mzmine": "passa", "criteri": []}
            pg.evaluate("(c) => { QTOOLS.ctx.files['cancello.json'] = new TextEncoder().encode(JSON.stringify(c)); PIPE.open(); }", gate)
            pg.wait_for_function("document.querySelectorAll('#pr-tab .pr-role').length === %d" % len(files), timeout=60000)
            pg.click("#pr-all"); pg.wait_for_function("PIPER.state.cs.every(c => c.tempo == null || c.tempo_confermato)", timeout=60000)
            try: pg.wait_for_function("!document.querySelector('#pipe-run-go').disabled", timeout=20000)
            except Exception: raise AssertionError(pg.evaluate("[PIPER.state.block, document.querySelector('#pr-warn').textContent, document.querySelector('#pipe-run-msg').textContent, PIPER.state.files]"))
            pg.click("#pipe-run-go")
            pg.wait_for_selector("#pipe-qc .qc #qc-m svg", timeout=600000)
            pg.wait_for_function("(document.querySelector('#ft-n') || {}).textContent && document.querySelector('#ft-n').textContent.includes(' of ')", timeout=120000)
            pg.click("#px-report")
            pg.wait_for_function("!document.querySelector('#pipe-exp .err')", timeout=60000)
            assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length"), "miss"
        step("Pipeline panel in the app: roles from the native engine, Run, quality control, table, report", pipeline_panel)
        def import_native():
            d = ROOT / "tests" / "data" / "untargeted"
            tab, ann, mgf = ((d / "mzmine" / n).read_text(encoding="utf-8") for n in ("tabella_pos.csv", "annotazioni_pos.csv", "fbmn.mgf"))
            r = pg.evaluate("""async ([tab, ann, mgf]) => {
                await UNT.ready();
                const t = await UNT.build([{ name: 'tabella_pos.csv', text: tab }, { name: 'annotazioni_pos.csv', text: ann }], { pol: 1 });
                await UNT.prime({ features: [{ mz: 184.1332, pol: 1, ann: [{ formula: 'C10H17NO2', adduct: 'M+H' }] }, { mz: 301.1547, pol: 1, ann: [{ formula: 'C10H17NO2', adduct: 'M+H' }] }] }, 20);
                let err = ''; try { await UNT.build([{ name: 'fbmn.mgf', text: mgf }], { pol: 1 }); } catch (e) { err = e.key || String(e); }
                return { n: t.features.length, s: t.samples, mz: t.byId.get('1').mz, has99: t.byId.has('99'), ann: t.features[0].ann[0].cleanName, ok: UNT.checkStructure('C10H17NO2', 184.1332, 'M+H', 1).ok, bad: UNT.checkStructure('C10H17NO2', 301.1547, 'M+H', 1).ok, err };
            }""", [tab, ann, mgf])
            assert r["n"] == 6 and r["s"] == ["Campione_A_pos", "Campione_B_pos"] and r["mz"] == 184.1332 and not r["has99"] and r["ann"] == "Testamine", r
            assert r["ok"] is True and r["bad"] is False and r["err"] == "noTable", r
        step("tables of other programs: mzMine table and annotations read by the native route, structure checks in a batch, a reader error keeps its key", import_native)
        def grafo_native():
            eth = "\n  qqq\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  2  3  1  0  0  0  0\nM  END"
            r = pg.evaluate("""async ([eth]) => {
                const F = await import('/static/disegno-frammenti.js'), pool = (await import('/static/mzfinder/feat-desktop-pool.js')).createDesktopPool();
                const cuts = await F.cutsOf(eth), g = JSON.parse(await pool.analyse('CCO'));
                const part = await (await F.carica()).partial(eth, '+O');
                let bad = ''; try { await (await F.carica()).partial(eth, '-C9H99'); } catch (e) { bad = typeof e === 'string' ? JSON.parse(e).error_key : String(e); }
                return { n: cuts.length, f: cuts[0].sides[0].formula, formula: g.formula, groups: Array.isArray(g.groups), part: JSON.parse(part).formula, bad };
            }""", [eth])
            assert r["n"] == 2 and r["formula"] == "C2H6O" and r["groups"] and r["part"], r
            assert r["bad"].startswith("error.grafo"), r
        step("structure graph: the cuts of a molfile, a partial structure and the SMILES of a parent on the native route", grafo_native)
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
            close(run["tree"], G["tree"], "tree")
            close(run["calibration"], G["calibration"], "calibration")
            close(call("msn_tree"), run["tree"], "msn_tree")
            close(call("frag_tree"), G["frag_tree"], "frag_tree")
            ds = [call("detail", x["id"]) for x in run["rows"][:12]]
            rows = {x["id"]: x for x in run["rows"]}
            for d in ds:
                if not (d.get("ms2") and d["row"]["formula"]):
                    assert d["localization"] is None and rows[d["id"]]["region"] is None, d["id"]
                    close(call("frag_tree_compare", d["id"]), G["compare_none"], f"compare {d['id']}")
                    continue
                want = G["localization"][str(d["id"])]["l"]
                close(d["localization"], want, f"localization {d['id']}")
                close(d["ms2"]["evidence"], want.get("evidence", []), f"evidence {d['id']}")
                ok = want.get("ok")
                assert rows[d["id"]]["region"] == (want["region"] if ok else None) and rows[d["id"]]["margin"] == (round(want["margin"], 2) if ok else None), ("row", d["id"])
                close(call("frag_tree_compare", d["id"]), G["localization"][str(d["id"])]["cmp"], f"compare {d['id']}")
        step("run with an MSn file: tree, calibration, localization, frag_tree and frag_tree_compare (= the frozen Python answers)", msn_standard)
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
        def lr_ms2_mrm():
            import dati_sintetici as DS; from valida import lr_sintetici as LS; import finder_demo as demo
            rng = numpy.random.default_rng(3); pm = demo.PARENT_MZ + demo.SHIFT
            epi, std, t0 = (tmp / "lr" / n for n in ("lr_EPI.mzML", "lr_MRM-std.mzML", "lr_MRM-t0.mzML"))
            DS._write(epi, [DS._spec(i, 8.8 + 0.02 * i, numpy.array([pm - 61, pm - 44, pm - 17]), numpy.array([120., 500., 900.]) * rng.normal(1, 0.05), level=2, prec=pm, ce=30) for i in range(21)], [])
            LS.scrivi_mrm(std, 10.0, 1); LS.scrivi_mrm(t0, 3.0, 2)
            pg.set_input_files("#tp-in", [str(x) for x in (epi, std, t0)])
            pg.wait_for_function("[...document.querySelectorAll('#tp-files tr')].filter(r => /lr_(EPI|MRM)/.test(r.textContent)).length >= 3", timeout=300000)
            files = LR_FILES + [{"name": epi.name, "type": "sample", "time": None, "kind": "ms2"}, {"name": std.name, "type": "standard", "time": None, "kind": "mrm"}, {"name": t0.name, "type": "sample", "time": 0, "kind": "mrm"}]
            call = lambda fn, *a: pg.evaluate("([fn, a]) => window.TPCALL(fn, ...a)", [fn, list(a)])
            run = call("run", files, LR_PARENT, {}, "")
            assert run["rows"][0]["label"] == "progenitore" and [r["id"] for r in run["rows"]] == [r["id"] for r in lr_native["run"]["rows"]], "the rows change with MS2 and MRM files"
            m = run["mrm"]; assert m and len(m["rows"]) == 2 and len(m["transitions"]) >= 2, m          # the integration of every transition, no calibration line
            assert m["calibration"] is None and not [k for k in ("slope", "intercept") if k in json.dumps(m).lower()], "the program draws no calibration line"
            d = call("detail", run["rows"][0]["id"])
            assert d["ms2"] and d["ms2"]["scans"] > 0 and d["ms2"]["fragments"], d.get("ms2")           # the EPI scans of the parent, with their fragments
            t = call("frag_tree"); assert t["mode"] == "lr", t
            e = call("estimate", [f["name"] for f in LR_FILES if f["type"] != "blank"], str(demo.PARENT_MZ))
            e = json.loads(e) if isinstance(e, str) else e
            assert e and all(k in e for k in ("da", "rt_tol", "dead", "fwhm")), e
            assert e["drift"]["n"] >= 2 and e["rt_tol"] >= e["drift"]["value"], e                   # rt_tol = max(2 x FWHM, drift of the parent)
        step("a low-resolution series with MS2 EPI and MRM files: rows, mrm table, MS2 of the parent, tree and estimate", lr_ms2_mrm)
        def lr_no_formula():
            call = lambda fn, *a: pg.evaluate("([fn, a]) => window.TPCALL(fn, ...a)", [fn, list(a)])
            run = call("run", LR_FILES, {"name": "parent", "mz": demo_mz}, {}, "")
            assert run["rows"] and run["rows"][0]["label"] == "progenitore" and run["parent"]["formula"] == "", (run["parent"], run["rows"][:1])
        step("a low-resolution series with a parent known only by its m/z runs in the app", lr_no_formula)
        def clean():
            bad = [u for u in reqs if re.search(r"pyodide|wasm-feat|mzlab\.zip", u, re.I)]
            assert not bad, bad[:5]
        step("no request names pyodide or wasm-feat", clean)
        b.close()
finally:
    eng.terminate(); page.shutdown()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
