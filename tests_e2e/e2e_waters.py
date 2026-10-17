"""A Waters MassLynx .raw folder (tools/waters_sintetico.py, no laboratory data) loaded with «Load a folder»: agilent-d.js groups it as a run of the module `waters` (no engine reads it), the module writes its
mzML in OPFS and the file enters the list as any mzML: TIC of the MS function, a spectrum with the three peaks that were encoded, the MRM channels as chromatograms, nothing misses the engine (MZLAB_LR_MISS
empty), only the module `waters` is downloaded. Needs the site and the modules: python tools/build_site.py, tools/build_lettori_wasm.sh site/static (MZLAB_PIPE_SITE=<built site> reuses a site).
Run from the repo root: python3 tests_e2e/e2e_waters.py"""
import os, shutil, socket, subprocess, sys, tempfile, threading
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
import waters_sintetico

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

tmp = Path(tempfile.mkdtemp(prefix="waters_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)], check=True)
    from lib_sito import add_wasm; add_wasm(site)
    src = ROOT / "site" / "static" / "wasm-lettori"
    assert (src / "manifest.json").exists(), "build it: tools/build_lettori_wasm.sh site/static"
    shutil.copytree(src, site / "static" / "wasm-lettori", dirs_exist_ok=True)
N = 40
top = tmp / "serie"; waters_sintetico.write(top, "uno.raw", N)

PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for _ in range(100):
    try: socket.create_connection(("127.0.0.1", PORT), 1).close(); break
    except OSError: threading.Event().wait(0.1)
errs = []; asked = []
LOAD = """async () => {
  await window.MZLAB_RUST_READY; await new Promise(r => setTimeout(r, 300));
  const tic = await (await fetch("api/chrom?k=0&level=1")).json(), sp = await (await fetch("api/spectrum?k=0&rt0=1.4&rt1=1.6&level=1")).json();
  return { files: ST.files.map(x => x.name), tic, sp, miss: (window.MZLAB_LR_MISS || []).length, missl: window.MZLAB_LR_MISS, kept: CONV.files.has("uno.mzML") };
}"""
res = {}
try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=os.environ.get("MZLAB_CHROMIUM") or None)
        pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("console", lambda m: print("  console:", m.text[:200]) if m.type in ("error", "warning") else None)
        pg.on("request", lambda r: asked.append(r.url) if "wasm-lettori/" in r.url else None)
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("window.MZLAB_RUST_READY !== undefined", timeout=60000)
        def opens():
            pg.set_input_files("#pickdir", str(top))
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=120000)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
            res["r"] = pg.evaluate(LOAD)
            assert res["r"]["files"] == ["uno.mzML"] and res["r"]["kept"], res["r"]
        step("the MassLynx folder is converted by the module and listed as uno.mzML", opens)
        def tic():
            t = res["r"]["tic"]; y = t.get("y"); assert y and len(y) == N, (list(t)[:6], y and len(y))
            assert max(y) > 4 * min(y), y[:5]
        step("its TIC is the MS function (one point per scan, with the peak)", tic)
        def spectrum():
            sp = res["r"]["sp"]; ks = [k for k in sp if isinstance(sp[k], list) and len(sp[k]) >= 3]
            assert ks, sp
            mz = next(sp[k] for k in ks if sp[k] and 100 < sp[k][0] < 1000)
            assert len(mz) == 3 and abs(mz[0] - 150.1) < 0.06 and abs(mz[1] - 300.3) < 0.06 and abs(mz[2] - 450.4) < 0.06, mz
        step("a spectrum has the three encoded peaks", spectrum)
        def miss():
            assert res["r"]["miss"] == 0, res["r"]["missl"]
        step("MZLAB_LR_MISS is empty", miss)
        def one_module():
            fams = {u.split("wasm-lettori/")[1].split("/")[0] for u in asked}
            assert "waters" in fams and fams <= {"waters", "manifest.json"}, asked
        step("only the module of the family (waters) is downloaded", one_module)
        b.close()
finally:
    srv.terminate()
for s in steps: print("  ", s)
errs = [e for e in errs if "GL Driver" not in e]
print("   errors:", errs[:5] if errs else "none")
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
