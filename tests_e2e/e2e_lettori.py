"""Datasets of a maker read as they are (mzlab/web/rust-nativo.js, nativi.rs): a synthetic Agilent .D folder (tools/agilent_sintetico.py, no laboratory data) is listed as a sample, indexed straight from
its files (no mzML is made), answers the TIC route, and «Download .mzML» writes its mzML on request (one spectrum per scan); a folder that no reader knows is an error with a key. Needs the site and the
.wasm: python tools/build_site.py (the .wasm is copied from site/static/wasm or built). MZLAB_PIPE_SITE=<built site> reuses a site. Run from the repo root: python3 tests_e2e/e2e_lettori.py"""
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

tmp = Path(tempfile.mkdtemp(prefix="lettori_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
N = 120
folder = agilent_sintetico.write(tmp / "serie", "uno.D", N)
members = [(str(p.relative_to(folder)).replace(os.sep, "/"), base64.b64encode(p.read_bytes()).decode()) for p in sorted(folder.rglob("*")) if p.is_file()]

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []
LOAD = """async ([members, wrong]) => {
  const rb = await window.MZLAB_RUST_READY;
  const mem = (wrong ? [[ "x.D/a.txt", btoa("nothing") ]] : members).map(([name, b64]) => ({ name, blob: new Blob([Uint8Array.from(atob(b64), c => c.charCodeAt(0))]) }));
  try {
    const s = (await rb.nativeSamples(mem)).samples;
    const f = NATIVO.sample("uno.mzML", NATIVO.key("uno.D"), mem, 0, s[0].id, false);
    await upload([f], "data");
    return { ok: true, scans: s[0].scans, files: ST.files.map(x => x.name), native: rb.isNative("uno.mzML") };
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
        def folder_ok():
            r = pg.evaluate(LOAD, [members, False]); assert r["ok"], r
            assert r["scans"] == N and r["files"] == ["uno.mzML"] and r["native"], r
        step("an Agilent .D folder is listed as one sample and indexed from its files", folder_ok)
        def mzml_on_request():
            r = pg.evaluate("""async () => { const rb = await window.MZLAB_RUST_READY; const f = await rb.nativeToMzml("uno.mzML", 1 << 20, undefined, undefined); return (await f.text()); }""")
            assert r.startswith("<?xml") and r.rstrip().endswith("</mzML>"), r[:80]
            assert r.count("<spectrum ") == N, r.count("<spectrum ")
        step("the mzML is written on request, one spectrum per scan", mzml_on_request)
        def unknown():
            r = pg.evaluate(LOAD, [members, True]); assert not r["ok"] and r["key"] == "err.file.notAccepted", r
        step("a folder that no reader knows is an error with a key", unknown)
        def again():
            r = pg.evaluate("async () => (await window.MZLAB_RUST_READY).nativeSamples ? 1 : 0"); assert r == 1
        step("the worker still works after the error", again)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
