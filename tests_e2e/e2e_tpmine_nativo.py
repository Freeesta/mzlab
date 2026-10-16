"""mzFinder reads the samples of a maker's file (.wiff + .wiff.scan) as they are: its Full Scan, MS2 and MRM samples are listed from the .mzidx of the Data tab (version 2) and no mzML is written for
them; a sample with profile scans (mzFinder works on centroids) gets a centroid index apart, `<name>.cen.mzidx`, beside the Data tab's; no mzML is written for it either. The SRM traces of a sample come from the engine of the page (`rb.nativeSrm`).
Synthetic pairs of tools/wiff_riferimento.py. Needs the site and the .wasm (python tools/build_site.py + the CI build)."""
import shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

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
        subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm", "--config", 'target.wasm32-unknown-unknown.rustflags=["-C","target-feature=+simd128"]'], cwd=ROOT, check=True)
        subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(wasm.parent), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)
    else:
        print("  ", ("wasm", "FAIL site/static/wasm missing and no cargo + wasm-bindgen to build it")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="wiff_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def pyodide(reqs): return [u for u in reqs if "/pyodide/" in u or u.endswith("mzlab.zip")]
def names(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")
def shown(pg, n): pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=120000)

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000)
        pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.set_input_files("#pick", [str(D / n) for n in ["two.wiff", "two.wiff.scan", "serie.wiff", "serie.wiff.scan", "prodotto.wiff", "prodotto.wiff.scan", "one.wiff", "one.wiff.scan"]])
        shown(pg, 6)
        pg.evaluate("""async () => { const rb = await window.MZLAB_RUST_READY, f = rb.nativeToMzml; window.__made = []; rb.nativeToMzml = (n, ...a) => { window.__made.push(n); return f.call(rb, n, ...a); }; }""")
        def listed():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.wait_for_function("document.querySelector('#tp-files') && /serie\\s+Full scan/.test(document.querySelector('#tp-files').innerText)", timeout=90000)
            t = pg.inner_text("#tp-files")
            assert "serie" in t, t
        step("mzFinder lists the samples", listed)
        def no_mzml():
            made = pg.evaluate("window.__made")
            print("   made:", made)
            assert "two_s2.mzML" not in made, made                             # the MRM sample is read as it is
            assert not any(n.startswith("prodotto") for n in made), made    # centroid samples are read as they are
            assert "serie.mzML" not in made, made                              # a profile sample is reduced to centroids by its own index, not through an mzML
        step("no mzML is written for a centroid, MRM or profile sample", no_mzml)
        def cen():
            l = pg.evaluate("""async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx"), o = []; for await (const [n] of d.entries()) o.push(n); return o; }""")
            assert "serie.mzML.cen.mzidx" in l and "serie.mzML.mzidx" in l, l       # the centroid index is apart; the Data tab's stays
            assert not any(n.startswith("prodotto") and ".cen." in n for n in l), l  # a centroid sample needs none
        step("a profile sample gets a centroid index beside the Data tab's", cen)
        def traces():
            r = pg.evaluate("""async () => { const rb = await window.MZLAB_RUST_READY, b = await rb.nativeSrm("two_s2.mzML"); return [b.constructor.name, new DataView(b.buffer, b.byteOffset).getUint32(0, true)]; }""")
            assert r[0] == "Uint8Array" and r[1] >= 0, r
        step("the SRM traces of an MRM sample come from the engine of the page", traces)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
