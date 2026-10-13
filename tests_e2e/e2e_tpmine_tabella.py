"""mzFinder, table of the products (TP_Mine/js/42_tp_table.js, 21_tpmine_hr.js): after the Dig of a synthetic HR series the candidates are a real table with sortable
headers, a filter, one expandable row at a time (the sheet #tp-det opens right under the row) and the t0 / blank columns from the Rust route `tp_table`. Keyboard: Enter on the
row button opens, arrows move, Esc closes. No pyodide request. Builds the site like e2e_tpmine_rust.py (MZLAB_PIPE_SITE reuses one). Run from the repo root: python3 tests_e2e/e2e_tpmine_tabella.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build crates/mzlab-feat-wasm"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)

tmp = Path(tempfile.mkdtemp(prefix="tpmine_tab_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    subprocess.run(["tools/build_feat_wasm.sh", str(site / "static")], cwd=ROOT, check=True, env=dict(os.environ, RUNNER_TEMP=str(tmp / "rt")), capture_output=True)
else:                                                      # fresh sources of mzFinder over the site that was built
    for f in (ROOT / "TP_Mine" / "files").glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
ser = tmp / "ser"
subprocess.run([sys.executable, "tools/feat_mirato.py", "--seme", "2", "--profilo", "piccolo", "--scala", "1.0", "--out", str(ser)], cwd=ROOT, check=True, capture_output=True)
idx = sorted(ser.glob("*.mzidx"))
data = site / "static" / "dati_finder"; data.mkdir(exist_ok=True)
for f in idx:
    shutil.copy2(f, data / (f.stem + ".mzML.mzidx"))      # the name the Data tab gives the index of «x.mzML»
    shutil.copy2(f.with_suffix(".mzML"), data / (f.stem + ".mzML"))
names = [f.stem + ".mzML" for f in idx]
PARENT = {"name": "parent", "neutral": "C21H32N2O2", "adduct": "[M+H]+"}
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1", "-d", str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_finder/" + n + ".mzidx")).body.pipeTo(w); }
}"""
PUTF = PUT.replace('"idx"]', '"files"]').replace('n + ".mzidx", { create', 'n, { create').replace('n + ".mzidx")).body', 'n)).body')
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce").new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.mzfinder','1');localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_function("typeof window.TPCALL === 'function'", timeout=120000)
        pg.evaluate(PUT, names); pg.evaluate(PUTF, names)
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
        pg.click("#tp-page"); pg.wait_for_function("document.querySelectorAll('#tp-files tr').length >= 7", timeout=120000)
        pg.fill("#tp-mol", PARENT["neutral"]); pg.click("#tp-go"); pg.wait_for_selector("#hr-t tr.clk", timeout=300000)
        rows = pg.locator("#hr-t tbody tr.clk")
        n0 = rows.count()

        def structure():
            assert pg.locator("#hr-t").evaluate("t => t.tagName") == "TABLE" and n0 >= 1, n0
            heads = pg.locator("#hr-t thead th").all_inner_texts()
            for h in ("m/z", "RT", "t0", "Blank", "Kinetics"): assert any(h in x for x in heads), (h, heads)
            assert not any(x.strip() in ("Class", "Region", "Pred.") for x in heads), heads          # few columns by default
            pg.locator("#hr-colm summary").click(); pg.check('#hr-cols input[data-col="class"]')
            assert any("Class" in x for x in pg.locator("#hr-t thead th").all_inner_texts())
            pg.uncheck('#hr-cols input[data-col="class"]')
            assert pg.locator("#hr-t tr.clk svg[role=img]").count() >= 1, "kinetics sparkline"
        step("a real table: headers, sparkline, kinetics column", structure)

        def columns():
            pg.wait_for_function("!document.querySelector('#hr-t0f').disabled", timeout=60000)
            cells = pg.locator("#hr-t tbody tr.clk td").all_inner_texts()
            assert pg.locator("#hr-t0f").is_enabled() and pg.locator("#hr-blf").is_enabled()
        step("t0 and blank columns arrive from the Rust route (filters enabled)", columns)

        def expand():
            first = rows.first
            tg = first.locator("button.tpt-tg")
            if tg.get_attribute("aria-expanded") == "true": tg.click(); pg.wait_for_timeout(200)
            assert pg.locator("#hr-t tr.tpt-exp").count() == 0
            tg.click(); pg.wait_for_selector("#hr-t tr.tpt-exp #tp-det", timeout=30000)
            assert tg.get_attribute("aria-expanded") == "true"
            pg.wait_for_function("document.querySelector('#hr-t tr.tpt-exp #tp-det').innerText.length > 30", timeout=30000)
            ids = pg.evaluate("[...document.querySelectorAll('#hr-t tr.tpt-exp')].length"); assert ids == 1, ids
            assert pg.locator("#tp-det").count() == 1 and pg.locator("#hr-kin").count() == 1
            nxt = rows.nth(1) if n0 > 1 else None
            if nxt:
                nxt.locator("button.tpt-tg").click(); pg.wait_for_timeout(300)
                assert pg.locator("#hr-t tr.tpt-exp").count() == 1 and tg.get_attribute("aria-expanded") == "false"
        step("a row opens its sheet right under it, one at a time", expand)

        def sort():
            col = pg.locator("#hr-t thead th", has_text="RT").first
            col.locator("button").click(); pg.wait_for_timeout(200)
            a = [float(x) for x in pg.eval_on_selector_all("#hr-t tbody tr.clk", "rs=>rs.map(r=>r.children[%d].innerText)" % pg.evaluate("[...document.querySelectorAll('#hr-t thead th')].findIndex(t=>/RT/.test(t.innerText))"))]
            d1 = col.get_attribute("aria-sort"); assert d1 in ("ascending", "descending"), d1
            assert a == sorted(a, reverse=d1 == "descending"), (d1, a)
            col.locator("button").click(); pg.wait_for_timeout(200)
            assert col.get_attribute("aria-sort") == ("ascending" if d1 == "descending" else "descending")
        step("sorting by a header (aria-sort)", sort)

        def filt():
            before = pg.locator("#hr-t tbody tr.clk").count()
            pg.check("#hr-ms2f"); pg.wait_for_timeout(300)
            after = pg.locator("#hr-t tbody tr.clk").count(); assert after <= before, (before, after)
            pg.uncheck("#hr-ms2f"); pg.wait_for_timeout(300)
            assert pg.locator("#hr-t tbody tr.clk").count() == before
            assert pg.inner_text("#hr-cnt").strip() != ""
        step("the MS2 filter narrows and restores the rows", filt)

        def keys():
            tgs = pg.locator("#hr-t button.tpt-tg")
            tgs.first.focus(); pg.keyboard.press("Enter"); pg.wait_for_selector("#hr-t tr.tpt-exp", timeout=30000)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.locator("#hr-t tr.tpt-exp").count() == 0
            assert pg.evaluate("document.activeElement.classList.contains('tpt-tg')")
            if tgs.count() > 1:
                pg.keyboard.press("ArrowDown")
                assert pg.evaluate("document.activeElement === document.querySelectorAll('#hr-t button.tpt-tg')[1]")
        step("keyboard: Enter opens, Esc closes, arrows move", keys)

        def sizes():
            bb = pg.locator("#hr-t button.tpt-tg").first.bounding_box(); assert bb["width"] >= 24 and bb["height"] >= 24, bb
        step("toggle target at least 24x24", sizes)

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
        step("no pyodide* request", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
