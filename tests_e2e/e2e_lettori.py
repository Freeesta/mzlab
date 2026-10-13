"""Datasets of a maker through the readers of the registry (mzlab/web/rust-lettori.js, crates/mzlab-lettori-wasm): a synthetic Agilent .D folder (tools/agilent_sintetico.py, no laboratory
data) becomes an mzML in the worker with `convertLettori`; the .wasm of the readers is fetched from static/wasm-feat/ only then, the blob has one spectrum per scan, a folder that no reader knows is
an error with a key, and a damaged manifest SHA-256 refuses the .wasm. Builds the site and static/wasm-feat (tools/build_feat_wasm.sh); SKIP without cargo and wasm-bindgen.
MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses a site. Run from the repo root: python3 tests_e2e/e2e_lettori.py"""
import base64, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import agilent_sintetico

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-lettori-wasm"); sys.exit(0)
tmp = Path(tempfile.mkdtemp(prefix="lettori_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")), capture_output=True)
N = 120
folder = agilent_sintetico.write(tmp / "serie", "uno.D", N)
members = [(str(p.relative_to(folder)).replace(os.sep, "/"), base64.b64encode(p.read_bytes()).decode()) for p in sorted(folder.rglob("*")) if p.is_file()]

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
CONVERT = """async ([members, wrong]) => {
  const rb = await window.MZLAB_RUST_READY;
  const files = members.map(([name, b64]) => ({ name, blob: new Blob([Uint8Array.from(atob(b64), c => c.charCodeAt(0))]) }));
  try {
    const blob = await rb.convertLettori("uno.D", wrong ? [{ name: "x.D/a.txt", blob: new Blob(["nothing"]) }] : files, p => {}, undefined, {});
    return { ok: true, text: await blob.text() };
  } catch (e) { return { ok: false, key: e.json && e.json.error_key, message: String(e.message) }; }
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page(); reqs = []
        pg.on("request", lambda r: reqs.append(r.url))
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
        def lazy():
            assert not [u for u in reqs if "wasm-feat/lettori" in u], "the readers are fetched only when a dataset is opened"
        step("the .wasm of the readers is not fetched before a dataset of a maker", lazy)
        def folder_ok():
            r = pg.evaluate(CONVERT, [members, False]); assert r["ok"], r
            t = r["text"]
            assert t.startswith("<?xml") and t.rstrip().endswith("</mzML>"), t[:80]
            assert t.count("<spectrum ") == N, t.count("<spectrum ")
            assert [u for u in reqs if "wasm-feat/lettori" in u], "the .wasm of the readers was fetched"
        step("an Agilent .D folder becomes an mzML with one spectrum per scan", folder_ok)
        def unknown():
            r = pg.evaluate(CONVERT, [members, True]); assert not r["ok"] and r["key"] == "err.file.notAccepted", r
        step("a folder that no reader knows is an error with a key", unknown)
        def again():
            r = pg.evaluate(CONVERT, [members, False]); assert r["ok"], r
        step("the worker still works after the error", again)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
