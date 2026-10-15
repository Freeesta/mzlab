"""Tabui takes the files of an instrument directly: «Choose files...» accepts .wiff + .wiff.scan (and .raw) besides mzML. The Data tab prepares them (one copy in OPFS, the .mzidx v2, the bar of
the phases), they appear in its list too, and Tabui receives the stubs as «Reload from Data» does: no mzML is written, no second archive, and «Reload from Data» afterwards does not rebuild any index.
Synthetic pairs of tools/wiff_riferimento.py. Needs the site and the .wasm (python tools/build_site.py + the CI build). Run from the repo root: python3 tests_e2e/e2e_tpmine_diretto_nativo.py"""
import shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
from lib import ROOT

PORT = 8838
errs = []
def step(name, fn):
    try: fn(); print("  ", (name, "ok"))
    except Exception as e: print("  ", (name, "FAIL " + str(e)[:600]))

if not (ROOT / "site" / "index.html").exists():
    print("  ", ("site", "FAIL site/ not built: python tools/build_site.py")); sys.exit(0)
if not (ROOT / "site" / "static" / "wasm" / "mzlab_wasm.js").exists():
    print("  ", ("wasm", "FAIL site/static/wasm missing")); sys.exit(0)

D = Path(tempfile.mkdtemp(prefix="wiff_e2e_"))
subprocess.run([sys.executable, str(ROOT / "tools" / "wiff_riferimento.py"), str(D)], check=True, capture_output=True)
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(ROOT / "site")], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1)
PAIRS = [str(D / n) for n in ["serie.wiff", "serie.wiff.scan", "prodotto.wiff", "prodotto.wiff.scan"]]
LIST_IDX = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx"), o = {}; for await (const [n, h] of d.entries()) o[n] = (await h.getFile()).lastModified; return o; }"""
LIST_FILES = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("files"), o = []; for await (const [n] of d.entries()) o.push(n); return o.sort(); }"""

try:
    with sync_playwright() as p:
        b = p.chromium.launch(executable_path=__import__("os").environ.get("MZLAB_CHROMIUM") or None)
        c = b.new_context(viewport={"width": 1500, "height": 1800}); c.set_default_timeout(120000)
        pg = c.new_page()
        pg.on("pageerror", lambda e: errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1")
        pg.wait_for_selector("#drop", timeout=120000); pg.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        pg.evaluate("""async () => { const rb = await window.MZLAB_RUST_READY; window.__calls = []; for (const k of ["nativeToMzml", "nativeCentroidIndex"]) { const f = rb[k]; rb[k] = (n, ...a) => { window.__calls.push(k + ":" + n); return f.call(rb, n, ...a); }; }
            window.__phases = new Set(); addEventListener("mzlab-index", e => window.__phases.add(e.detail.phase)); }""")
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        def label():
            t = pg.inner_text("#tp-pick")
            assert t.strip() == "Choose files...", t
            acc = pg.get_attribute("#tp-in", "accept")
            assert all(x in acc for x in (".raw", ".wiff", ".mzML")), acc
        step("one general button, accepts .raw .wiff .mzML", label)
        pg.set_input_files("#tp-in", PAIRS)
        def listed():
            pg.wait_for_function("/serie\\s+Full scan/.test((document.querySelector('#tp-files')||{}).innerText||'')", timeout=120000)
            t = pg.inner_text("#tp-files"); assert "serie" in t and "prodotto" in t, t
        step("the samples are listed in Tabui", listed)
        def in_data():
            n = pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=use]')).length")
            assert n >= 2, n
        step("they are in the list of the Data tab too (one archive)", in_data)
        def one_copy():
            f = pg.evaluate(LIST_FILES)
            assert sum(1 for x in f if x.startswith("nat~") and x.endswith("serie.wiff")) == 1, f      # one copy of the .wiff in OPFS
            made = pg.evaluate("window.__calls")
            assert not any(x.startswith("nativeToMzml") for x in made), made      # no mzML
        step("one copy in OPFS and no mzML", one_copy)
        def bar_gone():
            assert pg.inner_text("#tp-load").strip() == "", pg.inner_text("#tp-load")
            print("   phases seen:", pg.evaluate("[...window.__phases]"))
        step("the bars of the phases are gone when Tabui is ready", bar_gone)
        def reuse():
            before = pg.evaluate(LIST_IDX); pg.evaluate("window.__calls = []")
            pg.click("#tp-page"); pg.wait_for_function("/serie/.test(document.querySelector('#tp-files').innerText)", timeout=60000); time.sleep(1.5)
            after = pg.evaluate(LIST_IDX); calls = pg.evaluate("window.__calls")
            assert before == after, (before, after)                                # no index rebuilt
            assert not [x for x in calls if x.startswith("nativeToMzml")], calls
        step("Reload from Data rebuilds no index and writes no mzML", reuse)
        HASH = """async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx"), o = {}; for await (const [n, h] of d.entries()) if (/^(serie|prodotto)\\w*\\.mzML\\.mzidx$/.test(n)) { const u = new Uint8Array(await (await h.getFile()).arrayBuffer()); o[n] = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", u))).map(x => x.toString(16).padStart(2, "0")).join(""); } return o; }"""
        direct = pg.evaluate(HASH)
        def same_bytes():
            c2 = b.new_context(viewport={"width": 1500, "height": 1800}); c2.set_default_timeout(120000)
            p2 = c2.new_page(); p2.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
            p2.goto(f"http://127.0.0.1:{PORT}/?motore=rust&lr=1"); p2.wait_for_selector("#drop"); p2.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
            p2.set_input_files("#pick", PAIRS); p2.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=120000); time.sleep(1.5)
            data = p2.evaluate(HASH); c2.close()
            assert direct and direct == data, (direct, data)
        step("the .mzidx built from Tabui equals the one built from Data, byte for byte", same_bytes)
        assert not errs, errs
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(D, ignore_errors=True)
