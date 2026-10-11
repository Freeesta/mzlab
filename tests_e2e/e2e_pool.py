"""The indexes of the uploaded files are built by the pool of Rust workers (idx-worker.js, idxpool.js): same charts as with Python (?pool=0), the files appear
in the list as their index is ready, without the .wasm Python builds them as before.
Needs the site: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm)."""
import shutil, subprocess, sys, time
from playwright.sync_api import sync_playwright
from lib import ROOT, mz, free_port

wasm = ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js"
if not wasm.exists():
    if shutil.which("cargo") and shutil.which("wasm-bindgen"):
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "--config", 'target.wasm32-unknown-unknown.rustflags=["-C","target-feature=+simd128"]'], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

PORT = free_port()
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:300])); errs.append(name)

FILES = [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_FullMass-t60"), mz("B_MS2-t15")]
N = len(FILES)
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)

def session(b, query="", block_wasm=False):
    """Upload the files, return (page, engines of the indexes, the answers of chrom and xic after opening them)."""
    ctx = b.new_context(locale="it-IT", viewport={"width": 1400, "height": 1000})
    if block_wasm: ctx.route("**/static/wasm/**", lambda r: r.abort())
    pg = ctx.new_page()
    pg.on("pageerror", lambda e: errs.append("pageerror " + str(e)))
    pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
    pg.goto(f"http://127.0.0.1:{PORT}/{query}"); pg.wait_for_selector("#drop", timeout=120000)
    pg.set_input_files("#pick", FILES)
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {N}", timeout=180000)
    pg.wait_for_function(f"(window.MZLAB_IDX||[]).length >= {N}", timeout=60000)
    idx = pg.evaluate("window.MZLAB_IDX")
    pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000); pg.wait_for_timeout(1500)
    ans = pg.evaluate("Promise.all([fetch('api/chrom?k=0&kind=tic&level=1').then(r=>r.text()), fetch('api/xic?ks=0,1,2&mz=364&tol=0.5&level=1').then(r=>r.text())])")
    perf = pg.evaluate("fetch('api/perf').then(r=>r.json())")
    return pg, idx, ans, perf

try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        out = {}
        def pool():
            pg, idx, ans, perf = session(b, "?pool=auto")
            out["pool"] = (idx, ans)
            assert all(i["engine"] == "rust" and i["passes"] >= 1 for i in idx), idx
            assert all(f["indexed"] for f in perf["files"]), perf
            assert len({i["name"] for i in idx}) == N
        step("the pool builds every index with the Rust builder", pool)
        def python():
            pg, idx, ans, perf = session(b, "")
            out["py"] = (idx, ans)
            assert all(i["engine"] == "python" for i in idx), idx
        step("by default (and with ?pool=0) Python builds them: the pool is opt-in", python)
        def same():
            assert out["pool"][1] == out["py"][1], "chrom/xic differ between the Rust and the Python index"
        step("the charts from the Rust index are identical to those from the Python one", same)
        def nowasm():
            pg, idx, ans, perf = session(b, "?pool=auto", block_wasm=True)
            assert all(i["engine"] == "python" and i["why"] for i in idx), idx
            assert ans == out["py"][1]
        step("without the .wasm Python builds the indexes and the files open all the same", nowasm)
        def one():
            pg, idx, ans, perf = session(b, "?pool=1")
            assert all(i["engine"] == "rust" for i in idx) and ans == out["py"][1]
        step("a pool of one worker gives the same result", one)
        def forced_off():
            pg, idx, ans, perf = session(b, "?pool=0")
            assert all(i["engine"] == "python" for i in idx), idx
        step("?pool=0 gives Python", forced_off)
        b.close()
finally:
    srv.terminate()
sys.exit(1 if errs else 0)
