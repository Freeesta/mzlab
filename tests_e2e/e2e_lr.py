"""The files and the session of the browser version on the built site: opening Full Scan files, the list of the files, the session, the TIC and all the other routes
are answered by the Rust engine, Pyodide is never downloaded (the requests are counted, `MZLAB_LR_MISS` stays empty); the JSON equals the one of the Python reference
(pyref.py: mzlab/api.py in this process); the files are still there after a reload (OPFS).
Needs the site: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm)."""
import shutil, subprocess, sys, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT, mz
from golden_ref import PyRef

from lib import free_port; PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm"], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
FILES = [mz("B_FullMass-t0"), mz("B_FullMass-t15")]
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def json_close(a, b, tol=1e-9, path=""):
    if isinstance(a, dict) and isinstance(b, dict):
        assert a.keys() == b.keys(), f"{path}: keys {sorted(a.keys() ^ b.keys())}"
        for k in a: json_close(a[k], b[k], tol, f"{path}.{k}")
    elif isinstance(a, list) and isinstance(b, list):
        assert len(a) == len(b), f"{path}: length {len(a)} against {len(b)}"
        for i, (x, y) in enumerate(zip(a, b)): json_close(x, y, tol, f"{path}[{i}]")
    elif isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        assert abs(a - b) <= tol * max(1.0, abs(a), abs(b)), f"{path}: {a} against {b}"
    else: assert a == b, f"{path}: {a!r} against {b!r}"
def pyodide(reqs): return [u for u in reqs if "/pyodide/" in u or u.endswith("mzlab.zip")]
def why(pg): return (pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_ERR || null"))

TIMES = {}
MAPMS = {}
CLOCK = "async u => { const t = performance.now(); await fetch(u); return performance.now() - t; }"
def open_files(pg, label=None, files=None, n=2):
    t0 = time.time()
    pg.wait_for_selector("#drop", timeout=120000)
    if label: TIMES[label + " start→page ready"] = round(pg.evaluate("performance.now()") / 1000, 2)
    t0 = time.time()
    pg.set_input_files("#pick", files or FILES)
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function(f"E.files.length >= {n} && E.pending.length === 0", timeout=120000)
    if label: TIMES[label + " open"] = round(time.time() - t0, 2)
def snap(pg): return {"state": pg.evaluate(JSON, "api/state")["j"], "session": pg.evaluate(JSON, "api/session")["j"]}

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        def context():
            c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
            c.on("request", lambda r: reqs.append(r.url)); c.add_init_script("window.MZLAB_TRACE = [];"); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
            return c, pg, reqs
        ctx, pg, reqs = context()
        pg.goto(f"http://127.0.0.1:{PORT}/")
        def boot():
            pg.wait_for_selector("#drop", timeout=120000)
            pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            assert not pyodide(reqs), (pyodide(reqs)[:3], why(pg))
        step("the page starts on the Rust engine: Pyodide is not requested", boot)
        def load():
            open_files(pg, "rust flow")
            pg.wait_for_function("window.MZLAB_RUST.stats.rust > 0", timeout=60000)
            assert not pyodide(reqs), (pyodide(reqs)[:3], why(pg))
        step("upload, list, session and TIC of two Full Scan files without Pyodide", load)
        lr_snap = {}
        def lr_state():
            lr_snap.update(snap(pg))
            assert len(lr_snap["state"]["files"]) == 2 and len(lr_snap["session"]["session"]) == 2
            assert not pyodide(reqs), (pyodide(reqs)[:3], why(pg))
        step("api/state and api/session answered by Rust", lr_state)
        def reload_keeps():
            pg.wait_for_function("fetch('api/notebook').then(r => r.json()).then(n => (n.session || []).length >= 2)", timeout=30000)   # the notebook is saved 0.5 s after the last change: a reload before it loses the session
            pg.reload(); pg.wait_for_selector(".pnl canvas", timeout=180000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)   # the session of the notebook is open again
            st = pg.evaluate(JSON, "api/state")["j"]
            assert [f["name"] for f in st["files"]] == [f["name"] for f in lr_snap["state"]["files"]], st["files"]
            json_close(st["files"], lr_snap["state"]["files"], path="files")
            json_close(pg.evaluate(JSON, "api/session")["j"]["session"], lr_snap["session"]["session"], path="resumed session")
            assert not pyodide(reqs), (pyodide(reqs)[:3], why(pg))
        step("after a reload the files are still there (OPFS), still without Pyodide", reload_keeps)
        # the Python reference on the same files: the JSON must be the same
        ref = PyRef(FILES, pg.evaluate("JSON.parse(JSON.stringify(E.files.filter(f => !f.gone).map(f => ({ file: f.file, label: f.label, time: f.time, type: f.type, conc: f.conc ?? null, cunit: f.cunit ?? null, rep: f.rep || undefined }))))"))         # the same samples as the page posted, in the shape the Python reference knows (time and base type)
        def no_msn(x):   # `msn` (infusion recognised from the index, `infusione.rs`) and `hr` (the loading table groups high-resolution files apart) are the engine's alone, wherever a file is described: the reference has no such fields
            if isinstance(x, dict): return {k: no_msn(v) for k, v in x.items() if k not in ("msn", "hr")}
            return [no_msn(v) for v in x] if isinstance(x, list) else x
        def classic():
            py = {"state": ref.get("api/state")["j"], "session": ref.get("api/session")["j"]}
            json_close(no_msn(lr_snap["state"]["files"]), py["state"]["files"], path="state.files")
            json_close(no_msn({k: v for k, v in lr_snap["state"].items() if k not in ("files", "workdir")}), {k: v for k, v in py["state"].items() if k not in ("files", "workdir")}, path="state")
            json_close(no_msn({k: v for k, v in lr_snap["session"].items() if k not in ("workdir", "commit")}), {k: v for k, v in py["session"].items() if k not in ("workdir", "commit")}, path="session")
        step("the JSON of state and session equals the Python reference", classic)
        def same_as_classic():
            urls = ["api/spectrum?k=0&rt0=0&rt1=1000&level=1&precursor=&merge=1", "api/spectrum?k=1&rt0=1&rt1=2.5&level=1&bin=0.5", "api/spectrum?k=0&rt0=3&rt1=3&level=1",
                    "api/fileinfo?k=0", "api/fileinfo?k=1", "api/ms2tr?k=0", "api/ms2tr?k=0,1", "api/scan_axis?k=0&level=1", "api/scan_axis?k=1&level=2", "api/contaminants",
                    "api/formula?f=C6H12O6", "api/formula?f=c9h10cl2n2o&adduct=%5BM%2BNa%5D%2B", "api/formula?f=Xx2", "api/gradient?k=0", "api/method_warnings"]
            for u in urls:
                a = pg3.evaluate(JSON, u); b2 = ref.evaluate(JSON, u)
                assert a["status"] == b2["status"], (u, a["status"], b2["status"])
                json_close(a["j"], b2["j"], path=u)
                if not u.startswith("api/gradient") and not u.startswith("api/method_warn"): assert a["eng"] == "rust", (u, a["eng"])
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
        ctx3, pg3, reqs3 = context()
        pg3.goto(f"http://127.0.0.1:{PORT}/")
        open_files(pg3)
        step("spectrum, formula, gradient: same answers as the reference, still without Pyodide", same_as_classic)
        def origin_same():
            for u in ["api/origin?k=0&mz=194.2&parent=364.37", "api/origin?k=1&mz=305.3&parent=364.37&rt0=13.5&rt1=15.2", "api/origin?k=0&mz=152.2&parent=364.37&formula=C9H10Cl2N2O"]:
                a = pg3.evaluate(JSON, u); b2 = ref.evaluate(JSON, u)
                assert a["status"] == b2["status"] == 200, (u, a["status"], b2["status"])
                assert a["eng"] == "rust", (u, a["eng"])
                for r in (a["j"], b2["j"]): r.pop("timing", None)                    # seconds of each stage: they differ by nature
                json_close(a["j"], b2["j"], 1e-6, u)
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
        step("origin: the evidence of Rust equals the reference's, still without Pyodide", origin_same)
        def map_same():
            for u in ["api/map?k=0", "api/map?k=1&bg=1", "api/map?k=0&ref=1&norm=max", "api/map?k=0&rt0=13&rt1=16&mz0=140&mz1=400&nrt=120&nmz=100", "api/mapsnap?k=0&rt=14.25&mz=364.4",
                      "api/mappunti?k=0&ref=1&pts=14.3,364.3;14.32,194.2", "api/mappunti3d?k=0&rt0=13&rt1=16&mz0=150&mz1=370&max=300"]:
                ta, tb = round(pg3.evaluate(CLOCK, u)), round(ref.evaluate(CLOCK, u))
                MAPMS[u[4:40]] = (ta, tb)
                a = pg3.evaluate(JSON, u); b2 = ref.evaluate(JSON, u)
                assert a["status"] == b2["status"] == 200, (u, a["status"], b2["status"])
                assert a["eng"] == "rust", (u, a["eng"])
                json_close(a["j"], b2["j"], 1e-6, u)
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
        step("map, snap, marked points and 3D sticks: Rust equals the reference's, still without Pyodide", map_same)
        def composition_same():
            obs = "233.0243:100;234.0276:7.5;235.0310:3.2"
            for u in ["api/composition?mz=233.0243&elements=C%3A0-20%2CH%3A0-40%2CN%3A0-5%2CO%3A0-8%2CS%3A0-2&tol=5&unit=ppm&max=10",
                      "api/composition?mz=233.0243&tol=3&unit=mda&rdb=0,20&n=even&rules=HC,LEWIS,SENIOR&ion=%5BM%2BNa%5D%2B&max=5",
                      f"api/composition?mz=233.0243&elements=C%3A0-20%2CH%3A0-40%2CN%3A0-5%2CO%3A0-8%2CS%3A0-2&obs={obs.replace(':', '%3A').replace(';', '%3B')}&frags=215.0137%3A50%3B189.0243%3A100"]:
                a = pg3.evaluate(JSON, u); b2 = ref.evaluate(JSON, u)
                assert a["status"] == b2["status"] == 200, (u, a["status"], b2["status"])
                assert a["eng"] == "rust", (u, a["eng"])
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
        step("composition: the formulas of Rust equal the reference's, still without Pyodide", composition_same)
        def subxic_same():
            import tempfile
            import numpy as np
            sys.path.insert(0, str(ROOT / "tools"))
            import dati_sintetici as ds
            hrf = Path(tempfile.mkdtemp()) / "hr.mzML"
            sys.path.insert(0, str(ROOT))
            from mzlab.chem.elements import ion_mz, mass, parse_formula
            par = ion_mz(mass(parse_formula(ds.HR_PARENT)), "[M+H]+")
            ds.hr_series(hrf, np.random.default_rng(5), [(par, 6.0, 0.07, 3e7), (par - 61.9896, 6.0, 0.07, 1e7), (par - 97.9765, 6.1, 0.07, 4e6)])
            ctx6, pg6, reqs6 = context(); pg6.goto(f"http://127.0.0.1:{PORT}/"); open_files(pg6, files=[str(hrf)], n=1)
            pg7 = PyRef([str(hrf)])
            clock = """async q => { const rb = window.MZLAB_RUST; if (!(await rb.ready([0]))) return null; return JSON.parse((await rb.rpc({ op: "subxic", slot: 0, k: 0, query: q })).info || "null"); }"""
            for q in ["f=C14H14F4N3O2S&ppm=5&min=150", "f=C14H14F4N3O2S&ppm=2&min=300&z=1"]:        # the engine of the page, asked directly: the page itself goes to Python at its first high-resolution spectrum
                a = pg6.evaluate(clock, q); b2 = pg7.evaluate(JSON, "api/subxic?k=0&" + q)
                assert a is not None and a["n_hit"] > 0 and b2["status"] == 200, (q, a and a["n_hit"], b2["status"])
                json_close(a, b2["j"], 1e-9, q)
        step("subxic: the sub-formulas of a high-resolution survey from the index, Rust equals the reference's", subxic_same)
        def profile_same():
            import tempfile
            import numpy as np
            sys.path.insert(0, str(ROOT / "tools"))
            import dati_sintetici as ds
            pf = Path(tempfile.mkdtemp()) / "prof.mzML"
            ds.full_scan_profile(pf, 10.0, np.random.default_rng(6))
            ctx8, pg8, reqs8 = context(); pg8.goto(f"http://127.0.0.1:{PORT}/"); open_files(pg8, files=[str(pf)], n=1)
            pg9 = PyRef([str(pf)])
            for u in ["api/spectrum?k=0&rt0=14&rt1=14.7&level=1&precursor=&bin=0.1", "api/spectrum?k=0&rt0=0&rt1=1000&level=1&merge=1", "api/spectra?k=0&i0=150&i1=160&level=1&bin=0.1",
                      "api/spectra?k=0&i0=100&i1=159&level=1&merge=1"]:
                ta, tb = round(pg8.evaluate(CLOCK, u)), round(pg9.evaluate(CLOCK, u))
                MAPMS[u[4:44]] = (ta, tb)
                a = pg8.evaluate(JSON, u); b2 = pg9.evaluate(JSON, u)
                assert a["status"] == b2["status"] == 200, (u, a["status"], b2["status"])
                assert a["eng"] == "rust" and (a["j"]["mode"] if "mode" in a["j"] else a["j"]["scans"][0]["mode"]) == "profile", (u, a["eng"], why(pg8))
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pyodide(reqs8), (pyodide(reqs8)[:3], why(pg8))
        step("profile spectra (window and single scans): Rust equals the reference's, still without Pyodide", profile_same)
        def stat_same():
            import numpy as np
            rng = np.random.default_rng(12)
            roles = ["blank", "blank"] + ["sample"] * 9
            times = [None, None, 0, 0, 0, 15, 15, 15, 60, 60, 60]
            m = np.round(rng.lognormal(10, 1.2, (40, 11)) * np.array([0.05, 0.05] + [1] * 9)).tolist()
            post = """async a => { const r = await fetch(a[0], { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(a[1]) }); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
            ctxa, pga, reqsa = context(); pga.goto(f"http://127.0.0.1:{PORT}/"); pga.wait_for_selector("#drop", timeout=120000)
            pgb = PyRef()
            pts = [{"file": f"f{k}", "time": t, "rt": 5.0, "area": 100.0 * (k + 1), "height": 9.0 * k} for k, t in enumerate([30, 0, 60, 15])]
            for u, body in [("api/stat/profile", {"points": pts, "metric": "area"}), ("api/stat/blank", {"matrix": m, "roles": roles, "factor": 3, "min_detect": 2}),
                            ("api/stat/reference", {"matrix": m, "roles": roles, "times": times}), ("api/stat/pca", {"matrix": m, "roles": roles, "groups": [None, None] + ["a"] * 3 + ["b"] * 3 + ["c"] * 3}),
                            ("api/stat/blank", {"matrix": m, "roles": ["sample"] * 11})]:
                a = pga.evaluate(post, [u, body]); b2 = pgb.evaluate(post, [u, body])
                assert a["status"] == b2["status"] and a["eng"] == "rust", (u, a["status"], b2["status"], a["eng"], why(pga))
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pyodide(reqsa), (pyodide(reqsa)[:3], why(pga))
        step("statistics (profile, blank, reference, PCA): Rust equals the reference's, without Pyodide", stat_same)
        def diff_same():
            full = ["api/chrom?k=1&ref=0", "api/chrom?k=1&ref=0&norm=tic&kind=bpc", "api/chrom?k=1&ref=0&norm=istd&istd=364.4&factor=0.5&clip=1&mz0=100&mz1=300",
                    "api/xic?k=0,1&ref=0&mz=364.4&tol=0.35", "api/xic?k=1&ref=0&mz=305.3&tol=0.5&norm=istd&istd=364.4&rttol=0.3", "api/xic?k=1&ref=0&mz=364.4&norm=foo",
                    "api/xic?k=1&ref=0&mz=364.4&factor=200"]
            for u in full:
                a = pg3.evaluate(JSON, u); b2 = ref.evaluate(JSON, u)
                assert a["status"] == b2["status"] and a["eng"] == "rust", (u, a["status"], b2["status"], a["eng"], why(pg3))
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
            import tempfile
            import numpy as np
            sys.path.insert(0, str(ROOT / "tools"))
            import dati_sintetici as ds
            d = Path(tempfile.mkdtemp())
            ds.mrm(d / "m1.mzML", 1.0, np.random.default_rng(3), 1); ds.mrm(d / "m2.mzML", 0.5, np.random.default_rng(13), 2)
            ctxm, pgm, reqsm = context(); pgm.goto(f"http://127.0.0.1:{PORT}/")
            pgn = PyRef([str(d / "m1.mzML"), str(d / "m2.mzML")])
            for g in (pgm,):
                g.wait_for_selector("#drop", timeout=120000)
                g.set_input_files("#pick", [str(d / "m1.mzML"), str(d / "m2.mzML")])
                g.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
                g.click("#opbtn"); g.wait_for_selector("canvas", timeout=120000)
                g.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
            for u in ["api/mrm?k=1&ref=0", "api/mrm?k=0,1&ref=1&norm=tic&factor=0.7&clip=1", "api/mrm?k=1&ref=0&norm=istd&istd=364.1&rttol=0.4", "api/mrm?k=1&ref=0&norm=foo"]:
                a = pgm.evaluate(JSON, u); b2 = pgn.evaluate(JSON, u)
                assert a["status"] == b2["status"] and a["eng"] == "rust", (u, a["status"], b2["status"], a["eng"], why(pgm))
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pyodide(reqsm), (pyodide(reqsm)[:3], why(pgm))
        step("reference difference (chrom, xic, mrm with ref): Rust equals the reference's, without Pyodide", diff_same)
        def qaqc_same():
            for u in ["api/qaqc?k=0,1&roles=sample,blank&mz=364.4&rt=14.3&ratio=1", "api/qaqc?k=1,0&roles=qc,qc&mz=194.2&rt=14.3&unit=ppm&tol=700&win=1",
                      "api/qaqc?k=0,1&roles=sample&mz=364.4&rt=14.3", "api/qaqc?k=0,1&roles=sample,dark&mz=305.3&rt=14.7&thr=10&ratio=1"]:
                a = pg3.evaluate(JSON, u); b2 = ref.evaluate(JSON, u)
                assert a["status"] == b2["status"] and a["eng"] == "rust", (u, a["status"], b2["status"], a["eng"], why(pg3))
                json_close(a["j"], b2["j"], 1e-9, u)
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
        step("QA/QC of an ion (api/qaqc): Rust equals the reference's, without Pyodide", qaqc_same)
        def mrm_alone():
            ctx4, pg4, reqs4 = context()
            pg4.goto(f"http://127.0.0.1:{PORT}/")
            pg4.wait_for_selector("#drop", timeout=120000)
            pg4.set_input_files("#pick", [mz("B_MRM-t0")])
            pg4.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pg4.click("#opbtn"); pg4.wait_for_selector("canvas", timeout=120000)
            pg4.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
            assert not pyodide(reqs4), (pyodide(reqs4)[:3], why(pg4))
            r = pg4.evaluate(JSON, "api/mrm?k=0"); assert r["status"] == 200 and r["eng"] == "rust" and r["j"]["files"][0]["transitions"], r["eng"]
            assert not pyodide(reqs4), (pyodide(reqs4)[:3], why(pg4))
        step("an MRM file opens without Pyodide: the transitions come from the Rust engine", mrm_alone)
        def five_files():
            # the flow of e2e13 in the Rust default: five files (Full Scan, MS2, MRM), reload, panels
            ctx5, pg5, reqs5 = context()
            ctx5.add_init_script("window.MZLAB_TRACE = [];")
            pg5.goto(f"http://127.0.0.1:{PORT}/")
            pg5.wait_for_selector("#drop", timeout=120000)
            five = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60"), mz("B_MS2-t15"), mz("B_MRM-t0")]
            pg5.set_input_files("#pick", five)
            pg5.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 5", timeout=120000)
            pg5.click("#opbtn"); pg5.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg5.wait_for_function("E.files.length >= 5 && E.pending.length === 0", timeout=120000)
            assert not pyodide(reqs5), ("opening five files started Pyodide", pg5.evaluate("window.MZLAB_LR_MISS || null"))      # the page adds the files one by one: a file added must not reset the open indexes of the others
            pg5.wait_for_function("fetch('api/notebook').then(r => r.json()).then(n => (n.session || []).length >= 5)", timeout=60000)   # the notebook is saved 0.8 s + 0.5 s after the last change of the page: a reload before it keeps only the files that were open then
            pg5.reload()
            pg5.wait_for_function("[...document.querySelectorAll('.pnl canvas')].some(c => c.checkVisibility())", timeout=180000)   # the first canvas of the page may be a hidden one
            pg5.wait_for_function("E.files.length >= 1", timeout=180000)
            assert not pyodide(reqs5), (pyodide(reqs5)[:3], pg5.evaluate("window.MZLAB_LR_MISS || null"))
            assert not why(pg5)[0], why(pg5)
            assert not errs, errs[:3]
        step("five files (Full Scan, MS2, MRM): the reload resumes the session without Pyodide", five_files)
        def methods_same():
            import tempfile
            import numpy as np
            sys.path.insert(0, str(ROOT / "tools"))
            import dati_sintetici as ds, dam_riferimento as dr
            d = Path(tempfile.mkdtemp()); ds.full_scan(d / "press.mzML", 0, np.random.default_rng(9), pressure=True)
            dr.build(d)
            ca, pa, ra = context(); pa.goto(f"http://127.0.0.1:{PORT}/"); open_files(pa, files=[str(d / "press.mzML")], n=1)
            pb = PyRef([str(d / "press.mzML")])
            for i, n in enumerate(["Q1 full scan.dam", "MRM prova.dam", "garbage.dam"], 1):         # one at a time: the order of the methods is the order of the uploads
                pb.upload(str(d / n))
                pa.set_input_files("#pick", [str(d / n)])
                pa.wait_for_function(f"fetch('api/state').then(r => r.json()).then(s => s.methods.length >= {i})", timeout=60000)
            POST = """async ([u, b]) => { const r = await fetch(u, { method: "POST", body: JSON.stringify(b) }); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
            sa, sb = pa.evaluate(JSON, "api/state")["j"], pb.evaluate(JSON, "api/state")["j"]
            assert len(sa["methods"]) == 3, sa["methods"]
            json_close(sa["methods"], sb["methods"], path="state.methods")
            ses = pa.evaluate(JSON, "api/session")["j"]["session"][0]
            assert ses["analog"], ses["analog"]
            i = ses["analog"][0]["i"]
            gets = ["api/method?k=0", "api/method_warnings", f"api/analog?k=0&i={i}&n=50", f"api/analog?k=0&i={i}", "api/analog?k=0&i=999", "api/delay?k=0", "api/chrom?k=0&kind=pda", "api/gradient?k=0"]
            for u in gets:
                a, b2 = pa.evaluate(JSON, u), pb.evaluate(JSON, u)
                assert a["status"] == b2["status"], (u, a["status"], b2["status"])
                if a["status"] == 200: json_close(a["j"], b2["j"], path=u)
                else: assert a["j"].get("error_key") == b2["j"].get("error_key"), (u, a["j"], b2["j"])
                assert a["eng"] == "rust", (u, a["eng"], why(pa))
            texts = [("gradient_parse", {"fmt": "csv", "text": "0,5\n5,95\n10,95\n11,5"}), ("gradient_parse", {"fmt": "csv", "text": "x"}), ("gradient_parse", {"fmt": "course"}),
                     ("gradient_parse", {"fmt": "chromeleon", "text": "0.000 %B = 5.0\n5.000 %B = 90.0\nFlow = 0.4\n10.000 %B = 90.0"}),
                     ("gradient", {"k": 0, "fmt": "csv", "text": "0,5\n6,95\n8,95"}), ("gradient", {"k": 0, "model": {"points": [{"t": 0, "b": 5}, {"t": 4, "b": 90}]}})]
            for path, body in texts:
                a, b2 = pa.evaluate(POST, ["api/" + path, body]), pb.evaluate(POST, ["api/" + path, body])
                assert a["status"] == b2["status"], (path, body, a, b2)
                if a["status"] == 200: json_close(a["j"], b2["j"], path=path)
                else: assert a["j"].get("error_key") == b2["j"].get("error_key"), (path, a["j"], b2["j"])
                assert a["eng"] == "rust", (path, body, why(pa))
            json_close(pa.evaluate(JSON, "api/gradient?k=0")["j"], pb.evaluate(JSON, "api/gradient?k=0")["j"], path="gradient after POST")
            assert not pyodide(ra), (pyodide(ra)[:3], why(pa))
            pa.reload(); pa.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)           # the methods survive a reload (OPFS)
            assert len(pa.evaluate(JSON, "api/state")["j"]["methods"]) == 3 and not pyodide(ra), why(pa)
            pa.evaluate(POST, ["api/remove", {"name": "garbage.dam"}])
            assert len(pa.evaluate(JSON, "api/state")["j"]["methods"]) == 2
        step("methods (.dam), analog traces, PDA, delay and gradient: Rust equals the reference's, without Pyodide", methods_same)
        def methods_hr_same():
            import tempfile
            import numpy as np
            sys.path.insert(0, str(ROOT / "tools"))
            import dati_sintetici as ds, dam_riferimento as dr
            d = Path(tempfile.mkdtemp()); ds.hr_dda(d / "hrdda.mzML", np.random.default_rng(5)); dr.build(d)
            ca, pa, ra = context(); pa.goto(f"http://127.0.0.1:{PORT}/"); open_files(pa, files=[str(d / "hrdda.mzML")], n=1)
            pb = PyRef([str(d / "hrdda.mzML")])
            for i, n in enumerate(["Q1 full scan.dam", "MRM prova.dam"], 1):
                pb.upload(str(d / n)); pa.set_input_files("#pick", [str(d / n)])
                pa.wait_for_function(f"fetch('api/state').then(r => r.json()).then(s => s.methods.length >= {i})", timeout=60000)
            for u in ["api/method?k=0", "api/method_warnings", "api/analog?k=0&i=999", "api/delay?k=0", "api/chrom?k=0&kind=pda", "api/gradient?k=0"]:
                a, b2 = pa.evaluate(JSON, u), pb.evaluate(JSON, u)
                assert a["status"] == b2["status"], (u, a["status"], b2["status"])
                if a["status"] == 200: json_close(a["j"], b2["j"], path=u)
                else: assert a["j"].get("error_key") == b2["j"].get("error_key"), (u, a["j"], b2["j"])
                assert a["eng"] == "rust", (u, a["eng"], why(pa))
            assert "scan_params" in pa.evaluate(JSON, "api/method?k=0")["j"]
            assert not pyodide(ra) and not why(pa)[0], why(pa)
        step("high-resolution session with methods (.dam): method, scan parameters, delay, PDA and gradient equal the reference's", methods_hr_same)
        def mrm_traces_same():
            import tempfile
            import numpy as np
            sys.path.insert(0, str(ROOT / "tools"))
            import dati_sintetici as ds
            d = Path(tempfile.mkdtemp()); ds.mrm(d / "mrm.mzML", 1.0, np.random.default_rng(3), 1)
            ca, pa, ra = context(); pa.goto(f"http://127.0.0.1:{PORT}/"); pa.wait_for_selector("#drop", timeout=120000)
            pa.set_input_files("#pick", [str(d / "mrm.mzML")]); pa.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pa.click("#opbtn"); pa.wait_for_selector("canvas", timeout=120000); pa.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
            pb = PyRef([str(d / "mrm.mzML")])
            for u in ["api/chrom?k=0", "api/chrom?k=0&kind=bpc", "api/chrom?k=0&kind=srm", "api/chrom?k=0&kind=none", "api/chrom?k=0&level=2&mz0=100&mz1=300&prec=200", "api/xic?k=0&mz=200&tol=0.35",
                      "api/xic?k=0,0&mz=200&tol=0.35&level=2", "api/filters?k=0", "api/scanlist?k=0", "api/scan_axis?k=0"]:
                a, b2 = pa.evaluate(JSON, u), pb.evaluate(JSON, u)
                assert a["status"] == b2["status"] == 200, (u, a["status"], b2["status"], why(pa))
                json_close(a["j"], b2["j"], path=u)
                assert a["eng"] == "rust", (u, a["eng"], why(pa))
            assert len(pa.evaluate(JSON, "api/chrom?k=0")["j"]["rt"]) > 100
            assert not pyodide(ra) and not why(pa)[0], why(pa)
        step("an MRM file: its chromatograms and the empty routes of the scans equal the reference's, without Pyodide", mrm_traces_same)
        print("   MISURA (s, CI runner, 2 Full Scan files):", TIMES)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
