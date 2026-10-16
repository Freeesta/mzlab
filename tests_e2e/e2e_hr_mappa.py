"""High-resolution files with the browser version on the built site: the maps (api/map in the grid of the session, in a region
with cells of equal ppm, minus a reference; api/mapsnap, api/mappunti, api/mappunti3d) and the evidence of origin (api/origin) come from the Rust engine,
no request of those routes is left to Python (window.MZLAB_LR_MISS) and Pyodide is not downloaded; the JSON equals the Python reference's (pyref.py).
Needs the site and the .wasm (python tools/build_site.py)."""
from pathlib import Path
import subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT
from golden_ref import PyRef
from lib_hr import synth

sys.path.insert(0, str(ROOT))
from lib import free_port; PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:700]))

if not (ROOT / "site" / "index.html").exists() or not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("site", "SKIP site/ or site/static/wasm not built")); sys.exit(0)
import dati_sintetici as ds  # noqa: E402
PARENT = ds.hr_parent_mz()
D = synth("hrmappa")
FILES = [D / "HR_DDA-Exploris-t30.mzML", D / "HR_DDA-Fusion-t30.mzML"]
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
CLOCK = "async u => { const t = performance.now(); await fetch(u); return performance.now() - t; }"
def json_close(a, b, tol=1e-6, path=""):
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
def left(pg): return [m for m in pg.evaluate("window.MZLAB_LR_MISS || []") if "api/map" in m or "api/origin" in m]
def open_files(pg):
    pg.wait_for_selector("#drop", timeout=120000)
    pg.set_input_files("#pick", [str(f) for f in FILES])
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(FILES)}", timeout=120000)
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
    pg.wait_for_function("E.files.length >= 4 && E.pending.length === 0", timeout=120000); pg.wait_for_timeout(2000)
TIMES = {}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        def context():
            c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
            c.on("request", lambda r: reqs.append(r.url)); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e)))
            return pg, reqs
        pg, reqs = context(); pg.goto(f"http://127.0.0.1:{PORT}/"); open_files(pg)
        pg2 = PyRef([str(f) for f in FILES])
        items = pg.evaluate(JSON, "api/session")["j"]["session"]
        ms1 = [k for k, it in enumerate(items) if "MS1" in it["label"] or it.get("kind") == "full"]
        assert len(ms1) == 2, [(it["label"], it.get("kind")) for it in items]
        a, c = ms1
        def same(urls, tol=1e-6):
            for u in urls:
                ta, tb = round(pg.evaluate(CLOCK, u)), round(pg2.evaluate(CLOCK, u)); TIMES[u[4:44]] = (ta, tb)
                ra, rb_ = pg.evaluate(JSON, u), pg2.evaluate(JSON, u)
                assert ra["status"] == rb_["status"] == 200, (u, ra["status"], rb_["status"])
                assert ra["eng"] == "rust", (u, ra["eng"])
                for r in (ra["j"], rb_["j"]): r.pop("timing", None)
                json_close(ra["j"], rb_["j"], tol, u)
            assert not pyodide(reqs), pyodide(reqs)[:3]
            assert not left(pg), left(pg)
        def maps():
            R = "rt0=13&rt1=15.5&mz0=300&mz1=420"
            same([f"api/map?k={a}", f"api/map?k={c}&bg=1&agg=mean", f"api/map?k={a}&ref={c}&norm=max", f"api/map?k={a}&{R}&nrt=100&nmz=800",
                  f"api/map?k={a}&{R}&nrt=100&nmz=600&mzscale=log&bg=1", f"api/map?k={a}&ref={c}&{R}&nrt=80&nmz=600&mzscale=log&mztol=5",
                  f"api/map?k={a}&rt0=14.2&rt1=14.4&mz0={PARENT - 0.02}&mz1={PARENT + 0.02}&nrt=40&nmz=400&mzscale=log&ref={c}&mztol=20"])
        step("high-resolution maps (grid of the session, region, equal ppm, minus a reference) from Rust, equal to the reference's, without Pyodide", maps)
        def points():
            same([f"api/mapsnap?k={a}&rt=14.3&mz={PARENT}", f"api/mappunti?k={a}&ref={c}&pts=14.3,{PARENT};9.0,305.0702&ppm=5",
                  f"api/mappunti3d?k={a}&rt0=14&rt1=14.6&mz0={PARENT - 1}&mz1={PARENT + 3}&max=300"])
        step("snap, marked points and 3D sticks of high-resolution files from Rust, equal to the reference's", points)
        def origin():
            same([f"api/origin?k={a}&mz=305.0702&parent={PARENT}", f"api/origin?k={c}&mz={PARENT + 1.00335}&parent={PARENT}&rt0=13.8&rt1=14.8&formula={ds.HR_PARENT}"], 1e-6)
        step("the evidence of origin of high-resolution files from Rust, equal to the reference's, without Pyodide", origin)
        def tiles():
            u = f"api/maptiles?k={a}&level=1&rt0=0&rt1=60&mz0=100&mz1=1000&nrt=200&nmz=300"
            r1 = pg.evaluate(JSON, u); assert r1["status"] == 200 and r1["eng"] == "rust", (r1["status"], r1["eng"])
            j = r1["j"]; n = j["nrt"] * j["nmz"]; assert 0 < j["nrt"] <= 200 and 0 < j["nmz"] <= 300, (j["nrt"], j["nmz"])
            raw = __import__("base64").b64decode(j["data"]); assert len(raw) == 4 * n, (len(raw), n)
            assert any(raw), "the pyramid has no cell"
            assert len(j["rt"]) == j["nrt"] + 1 and j["dmz"] == 0.25 * 2 ** j["tile_level"], (len(j["rt"]), j["dmz"], j["tile_level"])
            saved = pg.evaluate("""async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("tiles"); const o = []; for await (const [n, h] of d.entries()) o.push([n, (await h.getFile()).size]); return o; }""")
            assert any(n.endswith(".L1.mzt") and sz > 16 for n, sz in saved), saved
            r2 = pg.evaluate(JSON, u); r1["j"].pop("timing", None); r2["j"].pop("timing", None)
            assert r2["j"] == r1["j"], "the second answer differs"
            assert not left(pg), left(pg)
        step("api/maptiles from Rust: cells that fit the request, the pyramid saved in OPFS, the same answer again", tiles)
        b.close()
finally:
    srv.terminate()
if errs: print("  ", ("pageerror", "; ".join(errs)[:400]))
print("  ", ("ms rust/python", TIMES))
