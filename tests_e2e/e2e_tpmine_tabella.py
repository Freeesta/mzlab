"""mzFinder, table of the products (TP_Mine/js/42_tp_table.js, 21_tpmine_hr.js): after the Dig of a synthetic HR series the candidates are a real table with sortable
headers, filter chips, one expandable row at a time (the panel #tp-det with four tabs opens right under the row), t0 / blank from the Rust route `tp_table`, the traces of the row
from `tp_row`, and "More" with one open section. Keyboard: Enter on the row button opens, arrows move, Esc closes. No pyodide request. Builds the site like e2e_tpmine_rust.py (MZLAB_PIPE_SITE reuses one). Run from the repo root: python3 tests_e2e/e2e_tpmine_tabella.py"""
import json, os, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent; SHOTS = HERE / "shots"; SHOTS.mkdir(exist_ok=True)
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
srv = subprocess.Popen([sys.executable, str(Path(__file__).resolve().with_name("servi.py")), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1); errs = []; reqs = []
PUT = """async (names) => {
  let d = await navigator.storage.getDirectory();
  for (const p of ["mzlab", "idx"]) d = await d.getDirectoryHandle(p, { create: true });
  for (const n of names) { const w = await (await d.getFileHandle(n + ".mzidx", { create: true })).createWritable(); await (await fetch("static/dati_finder/" + n + ".mzidx")).body.pipeTo(w); }
}"""
PUTF = PUT.replace('"idx"]', '"files"]').replace('n + ".mzidx", { create', 'n, { create').replace('n + ".mzidx")).body', 'n)).body')
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); pg = b.new_context(viewport={"width": 1280, "height": 900}, reduced_motion="reduce", accept_downloads=True).new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: errs.append(str(e)))
        pg.on("console", lambda m: print("CONSOLE", m.text[:400]) if "tp_row" in m.text else None)
        pg.context.on("request", lambda r: reqs.append(r.url))
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
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
            for h in ("Lvl", "Identity", "m/z", "RT", "Trend", "Cells"): assert any(h in x for x in heads), (h, heads)
            assert len(heads) <= 7, heads
            assert not any(x.strip() in ("Class", "Region", "Prev.", "t0", "Blank") for x in heads), heads          # few columns by default
            pg.locator("#hr-tbl .tpt-colm summary").click(); pg.check('#hr-tbl input[data-vc="class"]')
            assert any("Class" in x for x in pg.locator("#hr-t thead th").all_inner_texts())
            pg.uncheck('#hr-tbl input[data-vc="class"]'); pg.locator("#hr-tbl .tpt-colm summary").click()
            assert pg.locator("#hr-t tr.clk svg[role=img]").count() >= 1, "trend sparkline"
            assert pg.locator("#hr-t tbody tr.clk .tpt-cells").count() >= 1, "cells"
            badge = pg.locator("#hr-t tbody tr.clk .lvb").first.inner_text(); assert badge.startswith("L") and badge[1:].isdigit(), badge
            assert pg.locator("#hr-t thead th.tpt-stk").count() == 2, "level and identity stay fixed"
            for old in ("#hr-lv", "#hr-ord", "#hr-loc", "#hr-ms2f", "#hr-t0f", "#hr-blf", "#hr-colm"): assert pg.locator(old).count() == 0, old
        step("a real table: few columns, level badge, trend, cells", structure)

        def columns():
            pg.wait_for_function("!document.querySelector('#hr-tbl [data-chip=not0]').hasAttribute('aria-disabled')", timeout=60000)
            assert not pg.locator("#hr-tbl [data-chip=noblank]").get_attribute("aria-disabled")
            before = pg.locator("#hr-t tbody tr.clk").count()
            pg.click("#hr-tbl [data-chip=not0]"); pg.click("#hr-tbl [data-chip=noblank]"); lib.attendi(pg, "document.querySelector('#hr-tbl [data-chip=noblank]').getAttribute('aria-pressed') === 'true'")
            assert pg.locator("#hr-t tbody tr.clk").count() <= before
            pg.click("#hr-tbl [data-chip=not0]"); pg.click("#hr-tbl [data-chip=noblank]"); lib.attendi(pg, "document.querySelector('#hr-tbl [data-chip=noblank]').getAttribute('aria-pressed') === 'false'")
            assert pg.locator("#hr-t tbody tr.clk").count() == before
        step("t0 and blank arrive from the Rust route (the two chips are enabled and filter)", columns)

        def expand():
            first = rows.first
            tg = first.locator("button.tpt-tg")
            if tg.get_attribute("aria-expanded") == "true": tg.click(); pg.wait_for_timeout(200)
            assert pg.locator("#hr-t tr.tpt-exp").count() == 0
            tg.click(); pg.wait_for_selector("#hr-t tr.tpt-exp #tp-det", timeout=30000)
            assert tg.get_attribute("aria-expanded") == "true"
            pg.wait_for_selector("#tp-det .tpr-ch svg", timeout=60000)
            ids = pg.evaluate("[...document.querySelectorAll('#hr-t tr.tpt-exp')].length"); assert ids == 1, ids
            assert [t for t in pg.locator("#tp-det [role=tab]").all_inner_texts()] == ["Overview", "Samples", "Identity", "Quality"]
            hb = pg.locator("#tp-det .tpt-head .lvb").first.inner_text(); assert hb.startswith("L"), hb
            assert pg.locator("#tp-det .tpt-head .tpr-words").inner_text().strip() != ""
            assert pg.locator("#tp-det .tpr-ov svg").count() == 2, ("XIC of the row (from the traces) and the spectrum", pg.locator("#tp-det .tpr-ov svg").count(), pg.inner_text("#tp-det")[:600])
            assert pg.locator("#tp-det svg .tr").count() >= 6, "the XIC has a trace for every file"
            h = pg.evaluate("document.querySelector('#tp-det').getBoundingClientRect().height"); assert h <= 420, ("panel height", h)
            pg.click("#tp-det [data-tab=samples]"); pg.wait_for_selector("#hr-kin", timeout=30000)
            assert pg.locator("#tp-det").count() == 1 and pg.locator("#hr-kin").count() == 1 and pg.evaluate("document.getElementById('hr-kin').width") > 0
            assert pg.locator("#tp-det table.tpr tbody tr").count() >= 6, "a row per file with its state"
            h = pg.evaluate("document.querySelector('#tp-det').getBoundingClientRect().height"); assert h <= 420, ("panel height, samples", h)
            pg.click("#tp-det [data-tab=identity]"); pg.wait_for_selector("#tp-det .tpr-tw tbody tr", timeout=30000)
            assert "Criteria" in pg.inner_text("#tp-det") or "✓" in pg.inner_text("#tp-det") or "✗" in pg.inner_text("#tp-det")
            pg.click("#tp-det [data-tab=quality]"); pg.wait_for_selector("#tp-det dl.tpr-q", timeout=30000)
            pg.click("#tp-det [data-tab=overview]"); pg.wait_for_selector("#tp-det .tpr-ch svg")
            nxt = rows.nth(1) if n0 > 1 else None
            if nxt:
                nxt.locator("button.tpt-tg").click(); pg.wait_for_timeout(300)
                assert pg.locator("#hr-t tr.tpt-exp").count() == 1 and tg.get_attribute("aria-expanded") == "false"
                assert pg.locator("#hr-t [aria-expanded=true]").count() == 1, "opening another row closes the first"
        step("a row opens its four-tab panel right under it, one at a time", expand)

        def sort():
            col = pg.locator("#hr-t thead th", has_text="RT").first
            col.locator("button").click(); lib.attendi(pg, "[...document.querySelectorAll('#hr-t thead th')].some(t => /RT/.test(t.innerText) && t.getAttribute('aria-sort') !== 'none')")
            a = [float(x) for x in pg.eval_on_selector_all("#hr-t tbody tr.clk", "rs=>rs.map(r=>r.children[%d].innerText)" % pg.evaluate("[...document.querySelectorAll('#hr-t thead th')].findIndex(t=>/RT/.test(t.innerText))"))]
            d1 = col.get_attribute("aria-sort"); assert d1 in ("ascending", "descending"), d1
            assert a == sorted(a, reverse=d1 == "descending"), (d1, a)
            col.locator("button").click(); lib.attendi(pg, "[...document.querySelectorAll('#hr-t thead th')].some(t => /RT/.test(t.innerText) && t.getAttribute('aria-sort') !== '%s')" % d1)
            assert col.get_attribute("aria-sort") == ("ascending" if d1 == "descending" else "descending")
        step("sorting by a header (aria-sort)", sort)

        def filt():
            before = pg.locator("#hr-t tbody tr.clk").count()
            pg.click("#hr-tbl [data-chip=ms2]"); lib.attendi(pg, "document.querySelector('#hr-tbl [data-chip=ms2]').getAttribute('aria-pressed') === 'true'")
            after = pg.locator("#hr-t tbody tr.clk").count(); assert after <= before, (before, after)
            assert pg.get_attribute("#hr-tbl [data-chip=ms2]", "aria-pressed") == "true"
            pg.click("#hr-tbl [data-chip=ms2]"); lib.attendi(pg, "document.querySelector('#hr-tbl [data-chip=ms2]').getAttribute('aria-pressed') === 'false'")
            assert pg.locator("#hr-t tbody tr.clk").count() == before
            assert " of " in pg.inner_text("#hr-cnt")
        step("the MS2 chip narrows and restores the rows", filt)

        def more():
            for sel in ("#hr-m-net", "#hr-m-funnel", "#hr-rta", "#hr-m-tree"):
                if pg.locator(sel).count(): pg.click(sel + " summary"); pg.wait_for_timeout(100)          # the RT alignment only exists when the series has anchors
            assert pg.evaluate("document.querySelectorAll('.hr-more details[open]').length") == 1, "one section of More open at a time"
            assert pg.evaluate("document.querySelector('#hr-m-tree').open")
            assert pg.locator("#hr-t").count() == 1 and pg.locator("#hr-mnet").count() == 1
        step("More: one section open at a time", more)

        def export_view():
            pg.locator("#hr-tbl .tpt-xm summary").click()
            with pg.expect_download(timeout=30000) as dl: pg.click('#hr-tbl [data-x="csv"]')
            txt = Path(dl.value.path()).read_text(encoding="utf-8-sig"); assert txt.startswith("# filters:") and "Lvl" in txt.splitlines()[1], txt[:200]
        step("Export view: the rows and columns that are seen", export_view)

        def pictures():
            for theme in ("light", "dark"):
                pg.evaluate("t => { document.documentElement.dataset.theme = t; }", theme)
                pg.locator("#hr-t button.tpt-tg").first.click(); pg.wait_for_selector("#tp-det .tpr-ch svg", timeout=60000)
                pg.screenshot(path=str(SHOTS / f"tpmine_hr_tabella_{theme}.png"))
                pg.locator("#hr-t button.tpt-tg").first.click()
            pg.set_viewport_size({"width": 390, "height": 800}); pg.wait_for_timeout(300)
            pg.locator("#hr-t button.tpt-tg").first.click(); pg.wait_for_selector("#tp-det .tpr-ch svg", timeout=60000)
            w = pg.evaluate("[document.querySelector('#tp-det').getBoundingClientRect().width, document.querySelector('#hr-t').closest('.tpt-wrap').clientWidth]"); assert w[0] <= w[1] + 2, w
            pg.screenshot(path=str(SHOTS / "tpmine_hr_tabella_390.png")); pg.set_viewport_size({"width": 1280, "height": 900})
            pg.locator("#hr-t button.tpt-tg").first.click()
        step("pictures: light, dark, 390 px", pictures)

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
