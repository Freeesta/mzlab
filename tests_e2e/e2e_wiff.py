"""SCIEX .wiff + .wiff.scan dropped in the start page, on the built site with the Rust engine and WITHOUT Pyodide (?motore=rust&lr=1): the pair is matched by name
(also when its two files come in different loadings), a .wiff alone gets a notice, a .wiff with several samples gives one mzML per sample, the converted mzML are
listed (with the button to save them), are recognised as Full Scan / MS2 / MRM and Pyodide is never requested. Synthetic pairs of tools/wiff_riferimento.py (no laboratory data).
Needs the site and the .wasm: python tools/build_site.py; the .wasm is built here when cargo and wasm-bindgen exist (the CI builds it in site/static/wasm)."""
import shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

PORT = 8836
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
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
JSON = """async u => { const r = await fetch(u); return { status: r.status, eng: r.headers.get("X-Engine"), j: await r.json() }; }"""
def pyodide(reqs): return [u for u in reqs if "/pyodide/" in u or u.endswith("mzlab.zip")]
def names(pg): return pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")
def shown(pg, n): pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {n}", timeout=120000)

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000); reqs = []
        c.on("request", lambda r: reqs.append(r.url)); pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        def alone():
            pg.set_input_files("#pick", [str(D / "one.wiff")])
            pg.wait_for_function("document.querySelector('#unopen') && !document.querySelector('#unopen').hidden", timeout=60000)
            t = pg.inner_text("#unopen")
            assert "one.wiff" in t and ".wiff.scan" in t, t
            assert pg.locator("#flist input[data-k=use]").count() == 0
        step("a .wiff alone gets a notice and nothing is loaded", alone)
        def late_scan():
            pg.set_input_files("#pick", [str(D / "one.wiff.scan")])
            shown(pg, 1)
            assert names(pg) == ["one.mzML"], names(pg)
        step("its .wiff.scan arrives in another loading: the pair is converted into one.mzML", late_scan)
        def several():
            pg.evaluate("window.__rows = new Set(); new MutationObserver(() => document.querySelectorAll('#flist tr[aria-busy=true]').forEach(tr => window.__rows.add(tr.children[1].textContent.trim()))).observe(document.getElementById('flist'), { childList: true, subtree: true })")
            pg.set_input_files("#pick", [str(D / "two.wiff.scan"), str(D / "two.wiff"), str(D / "serie.wiff"), str(D / "serie.wiff.scan")])
            shown(pg, 4)
            assert sorted(names(pg)) == ["one.mzML", "serie.mzML", "two_s1.mzML", "two_s2.mzML"], names(pg)
        step("a .wiff with two samples gives one mzML per sample (either order of the files)", several)
        def rows_while_converting():                     # each pair is in the list as soon as it is paired, then each sample, until its mzML is in the list
            seen = set(pg.evaluate("Array.from(window.__rows)"))
            assert {"two.wiff", "serie.wiff", "two_s1.mzML", "two_s2.mzML", "serie.mzML"} <= seen, seen
            assert pg.locator("#flist tr[aria-busy=true]").count() == 0 and pg.evaluate("CONVROWS.rows.length") == 0
        step("the pairs and the samples are listed (in conversion) before their mzML is ready", rows_while_converting)
        def rows_progress():                             # the bar of a row moves without drawing the list again
            pg.evaluate("CONVROWS.add('x_s1.mzML', 'wait'); CONVROWS.add('x_s2.mzML', 'wait'); CONVROWS.set('x_s1.mzML', 'run', 0)")
            bar = pg.locator("#flist [data-cvrow='x_s1.mzML'] [role=progressbar]")
            pg.evaluate("CONVROWS.set('x_s1.mzML', 'run', 40)")
            assert bar.get_attribute("aria-valuenow") == "40" and "40" in pg.locator("#flist [data-cvrow='x_s1.mzML'] [data-cvtxt]").inner_text()
            assert pg.locator("#flist [data-cvrow='x_s2.mzML'] [data-cvtxt]").inner_text().strip() != ""
            pg.evaluate("CONVROWS.drop('x_s1.mzML'); CONVROWS.drop('x_s2.mzML')")
            assert pg.locator("#flist tr[aria-busy=true]").count() == 0
        step("a row of the list shows its state and a bar that moves", rows_progress)
        def pool_converts():                             # the samples are converted by short-lived workers (wiff-worker.js), not by the engine's own thread
            assert any(u.endswith("wiff-worker.js") for u in reqs), [u for u in reqs if "worker" in u]
        step("the conversion runs in the pool of workers", pool_converts)
        def downloads():
            assert pg.locator("#flist [data-dl]").count() == 4 and pg.locator("#dlall").count() == 1
        step("the converted mzML can be saved", downloads)
        def kinds():
            st = pg.evaluate(JSON, "api/state")["j"]
            f = {x["name"]: x for x in st["files"]}
            assert set(f) == {"one.mzML", "serie.mzML", "two_s1.mzML", "two_s2.mzML"}, list(f)
            groups = pg.evaluate("Array.from(document.querySelectorAll('#flist tr.grp')).map(tr=>tr.textContent)")
            assert any("MRM" in g for g in groups) and any("Full Scan" in g for g in groups), (groups, st["files"][0])
        step("the experiment type is recognised (Full Scan and MRM groups)", kinds)
        def listed_without_python():
            assert not pyodide(reqs), pyodide(reqs)[:3]
        step("conversion and list: Pyodide was not requested", listed_without_python)
        def opens():
            for n in ("one.mzML", "two_s1.mzML", "two_s2.mzML"):        # tiny files and MRM chromatograms are not what this test shows
                pg.evaluate("n => Array.from(document.querySelectorAll('#flist tr:not(.grp)')).find(tr => tr.children[1].textContent.trim().startsWith(n)).querySelector('input[data-k=use]').click()", n)
            pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
            pg.wait_for_function("E.files.length >= 1 && E.pending.length === 0", timeout=120000)
            tic = pg.evaluate(JSON, "api/chrom?k=0&level=1")["j"]
            assert len(tic["rt"]) == 80 and max(tic["y"]) > 10 * min(tic["y"]), tic["rt"][:3]
            ses = pg.evaluate(JSON, "api/session")["j"]["session"]
            assert len(ses) == 1, ses
        step("the files open as a session with their chromatograms", opens)
        def lonely():
            pg.set_input_files("#pick", [str(D / "lonely.wiff")])
            pg.wait_for_function("document.querySelector('#unopen') && document.querySelector('#unopen').innerText.includes('lonely')", timeout=60000)
        step("a second .wiff without scan gets its own notice", lonely)
        def nopy():
            assert not pyodide(reqs), (pyodide(reqs)[:3], pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_ERR || null"))
            assert not errs, errs
        step("opening the Full Scan series without Pyodide, and no page error", nopy)
        def mzfinder():                                  # the converted files are ordinary files of the Data tab: mzFinder (LR) lists them
            pg.evaluate("localStorage.setItem('qqq.mzfinder', '1')"); pg.reload()
            pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=120000)
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            try: pg.wait_for_function("document.querySelector('#tp-files') && /serie\\s+Full scan/.test(document.querySelector('#tp-files').innerText)", timeout=60000)
            except Exception: raise AssertionError(pg.evaluate("[document.querySelector('#tp-files').innerText, document.querySelector('#tp-fsum').innerText, (document.querySelector('#tp-msg')||{}).innerText, Array.from(document.querySelectorAll('#flist tr')).length]"))
        step("mzFinder lists the converted Full Scan file", mzfinder)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
