"""Flow of the low-resolution files WITHOUT Pyodide (?motore=rust&lr=1) on the built site: opening Full Scan files, the list of the files, the session and the
TIC are answered by the Rust engine, Pyodide is not downloaded (the requests are counted); the JSON of api/state and api/session equals the one of the classic
engine (Pyodide); the files are still there after a reload (OPFS); the first request only Python can answer starts Pyodide and the session is replayed into it.
Needs the site: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm)."""
import shutil, subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, mz

PORT = 8834
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "--config", 'target.wasm32-unknown-unknown.rustflags=["-C","target-feature=+simd128"]'], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
def open_files(pg, label=None):
    t0 = time.time()
    pg.wait_for_selector("#drop", timeout=120000)
    if label: TIMES[label + " start→page ready"] = round(pg.evaluate("performance.now()") / 1000, 2)
    t0 = time.time()
    pg.set_input_files("#pick", FILES)
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 2 && E.pending.length === 0", timeout=120000)
    if label: TIMES[label + " open"] = round(time.time() - t0, 2)
def snap(pg): return {"state": pg.evaluate(JSON, "api/state")["j"], "session": pg.evaluate(JSON, "api/session")["j"]}

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        def context():
            c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
            c.on("request", lambda r: reqs.append(r.url)); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
            return c, pg, reqs
        ctx, pg, reqs = context()
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
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
            pg.reload(); pg.wait_for_selector(".pnl canvas", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)   # the session of the notebook is open again
            st = pg.evaluate(JSON, "api/state")["j"]
            assert [f["name"] for f in st["files"]] == [f["name"] for f in lr_snap["state"]["files"]], st["files"]
            json_close(st["files"], lr_snap["state"]["files"], path="files")
            json_close(pg.evaluate(JSON, "api/session")["j"]["session"], lr_snap["session"]["session"], path="resumed session")
            assert not pyodide(reqs), (pyodide(reqs)[:3], why(pg))
        step("after a reload the files are still there (OPFS), still without Pyodide", reload_keeps)
        # the classic engine on the same files: the JSON must be the same
        ctx2, pg2, reqs2 = context()
        pg2.goto(f"http://127.0.0.1:{PORT}/?motore=python")
        def classic():
            open_files(pg2, "classic")
            assert pyodide(reqs2), "the counter does not see Pyodide in the classic flow"
            py = snap(pg2)
            json_close(lr_snap["state"]["files"], py["state"]["files"], path="state.files")
            json_close({k: v for k, v in lr_snap["state"].items() if k != "files"}, {k: v for k, v in py["state"].items() if k != "files"}, path="state")
            json_close(lr_snap["session"], py["session"], path="session")
        step("the JSON of state and session equals the classic engine's (Pyodide)", classic)
        def late_python():
            open_files(pg) if not pg.evaluate("typeof S !== 'undefined' && !!S.sess") else None
            before = pg.evaluate(JSON, "api/session")["j"]["session"]
            r = pg.evaluate(JSON, "api/gradient?k=0")                   # a route only Python knows
            assert r["status"] == 200, r
            assert pyodide(reqs), "Pyodide did not start"
            assert pg.evaluate("window.MZLAB_LR_DONE === true")
            after = pg.evaluate(JSON, "api/session")["j"]["session"]
            json_close(before, after, path="session")
        step("the first request only Python can answer starts Pyodide and the session is replayed into it", late_python)
        def same_as_classic():
            urls = ["api/spectrum?k=0&rt0=0&rt1=1000&level=1&precursor=&merge=1", "api/spectrum?k=1&rt0=1&rt1=2.5&level=1&bin=0.5", "api/spectrum?k=0&rt0=3&rt1=3&level=1",
                    "api/formula?f=C6H12O6", "api/formula?f=c9h10cl2n2o&adduct=%5BM%2BNa%5D%2B", "api/formula?f=Xx2", "api/gradient?k=0", "api/method_warnings"]
            for u in urls:
                a = pg3.evaluate(JSON, u); b2 = pg2.evaluate(JSON, u)
                assert a["status"] == b2["status"], (u, a["status"], b2["status"])
                json_close(a["j"], b2["j"], path=u)
                if not u.startswith("api/gradient") and not u.startswith("api/method_warn"): assert a["eng"] == "rust", (u, a["eng"])
            assert not pyodide(reqs3), (pyodide(reqs3)[:3], why(pg3))
        ctx3, pg3, reqs3 = context()
        pg3.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        open_files(pg3)
        step("spectrum, formula, gradient: same answers as the classic engine, still without Pyodide", same_as_classic)
        def mrm_alone():
            ctx4, pg4, reqs4 = context()
            pg4.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
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
            pg5.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
            pg5.wait_for_selector("#drop", timeout=120000)
            five = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60"), mz("B_MS2-t15"), mz("B_MRM-t0")]
            pg5.set_input_files("#pick", five)
            pg5.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 5", timeout=120000)
            pg5.click("#opbtn"); pg5.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg5.wait_for_function("E.files.length >= 5 && E.pending.length === 0", timeout=120000); pg5.wait_for_timeout(2000)
            pg5.reload()
            pg5.wait_for_selector(".pnl canvas", timeout=60000)
            pg5.wait_for_function("E.files.length >= 1", timeout=60000)
            assert not pyodide(reqs5), pyodide(reqs5)[:3]
            assert not why(pg5)[0], why(pg5)
            assert not errs, errs[:3]
        step("five files (Full Scan, MS2, MRM): the reload resumes the session without Pyodide", five_files)
        print("   MISURA (s, CI runner, 2 Full Scan files):", TIMES)
        step("no page errors", lambda: [(_ for _ in ()).throw(AssertionError(str(errs[:3]))) for _ in [0] if errs])
        b.close()
finally:
    srv.terminate()
