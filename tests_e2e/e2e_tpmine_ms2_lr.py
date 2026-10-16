"""Tabui, low resolution: the card «MS2 matches» (TP_Mine/js/22_tpmine_ms2.js). A synthetic Full Scan series (TP_Mine/tests/lr_synth.py) and one MS2 file with four
experiments (write_ms2): two on the first isomer of the +O product and the second isomer (two CE), one at a time with no signal, one on the parent. Opened in the Data tab, dug from
Tabui: every experiment is in the card with its state or its reason; a menu moves an MS2 to another row (no new Dig), the detail of that row shows its spectra, the
correction survives a reload and a new Dig, «none» and «Reset matches» work, the Excel export has the sheet «MS2 matches», the card works from the keyboard and has no axe
violation, no pyodide* request, `MZLAB_LR_MISS` empty. Builds the site and static/wasm-feat (SKIP without cargo, wasm-bindgen or numpy); MZLAB_PIPE_SITE=<built site with
static/wasm-feat> reuses one; AXE_JS=<axe.min.js> turns on the axe step. Shots: MZLAB_PIPE_SHOTS=<folder> (light and dark, 1280 and 390 px). Run from the repo root."""
import json, os, shutil, subprocess, sys, tempfile, time, zipfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402  (patches wait_for_function for the CSP)

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:900]))

if not (shutil.which("cargo") and shutil.which("wasm-bindgen")):
    print("SKIP cargo and wasm-bindgen are needed to build the wasm of the engine and of mzFinder"); sys.exit(0)
try:
    import numpy  # noqa: F401
except ImportError:
    print("SKIP numpy is needed for the synthetic series"); sys.exit(0)
sys.path.insert(0, str(ROOT / "TP_Mine" / "tests")); import lr_synth  # noqa: E402

tmp = Path(tempfile.mkdtemp(prefix="tpmine_ms2_lr_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    if os.environ.get("PYODIDE_DIR"): cmd += ["--pyodide-dir", os.environ["PYODIDE_DIR"]]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:                                                      # fresh sources of mzFinder over the site that was built
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
ser = tmp / "ser"
series = lr_synth.write_series(ser, times=(0, 15, 30, 60), blank=False)
EXPS = [{"precursor": 257.06, "ce": 10.0, "peaks": [(7.4, 0.07, 6e5)]},                     # the first +O isomer (RT 7.4)
        {"precursor": 257.06, "ce": 25.0, "peaks": [(8.3, 0.07, 6e5)]},                     # the second one, two collision energies
        {"precursor": 257.06, "ce": 40.0, "peaks": [(8.3, 0.07, 6e5)], "fragments": [(-17.0, 0.3), (-44.0, 1.0), (-60.0, 0.4)]},
        {"precursor": 333.20, "ce": 15.0, "peaks": []},                                     # a row at 6.8 min, a flat trace: no signal
        {"precursor": 241.06, "ce": 10.0, "peaks": [(9.0, 0.07, 6e5)]}]                     # the parent (RT 9.0): the products are compared with its MS2
ms2 = lr_synth.write_ms2(ser, EXPS, 30.0, name="lr_MS2-t30.mzML")
paths = [str(f["path"]) for f in series] + [str(ser / ms2["name"])]
SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; reqs = []
import urllib.request
for _ in range(100):                      # the server answers: wait on that, not on a pause
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).close(); break
    except Exception: srv.poll()
FIRST = "Row for the MS2 of m/z 257.06 at RT 7.40"            # the menu of the first experiment
TABLE = """() => [...document.querySelectorAll('#tp-ms2map table tr')].filter(r => r.querySelector('td')).map(r => [...r.children].map(c => (c.querySelector('select') ? c.querySelector('select').selectedOptions[0].textContent : c.textContent).trim()))"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); ctx = b.new_context(viewport={"width": 1280, "height": 1000}, reduced_motion="reduce"); pg = ctx.new_page()
        try: ctx.grant_permissions(["clipboard-read", "clipboard-write"])
        except Exception: pass
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.on("pageerror", lambda e: errs.append(str(e)))
        ctx.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.set_input_files("#pick", paths)
        pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=240000)

        def tab_and_dig():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length > 1", timeout=60000)
            txt = pg.evaluate("document.getElementById('tp-files').textContent")
            assert "MS2" in txt and "fragments" in txt, "the automatic start dropped the MS2 file: " + txt[:300]
            try: pg.wait_for_function("document.querySelector('#tp-tol').classList.contains('tp-est')", timeout=60000)      # the estimate of the settings holds the indexes until it is done
            except Exception: pass
            pg.fill("#tp-mol", SMILES)
            pg.click("#tp-go"); pg.wait_for_selector("#tp-ms2map", state="attached", timeout=360000)
        def table_view():                                  # the view «Table», the section «MS2 matches» open and the optional column «MS2» on
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t tr.clk")
            pg.evaluate("document.getElementById('tp-m-ms2').open = true")
            if not pg.locator('#tp-t thead th', has_text="MS2").count():
                pg.click(".tpt-colm summary"); pg.check('[data-vc="ms2"]'); pg.click(".tpt-colm summary")
            assert pg.locator('#tp-t thead th', has_text="MS2").count() == 1
        def dig():
            tab_and_dig()
            table_view()
        step("the automatic start keeps the MS2 file; the Dig gives the card «MS2 matches»", dig)

        def card():
            t = pg.evaluate(TABLE)
            fam = {(r[1], r[2]) for r in t}
            assert {("257.06", "10"), ("257.06", "25"), ("257.06", "40"), ("333.20", "15")} <= fam, fam           # every experiment is listed
            first = next(r for r in t if r[1] == "257.06" and r[2] == "10" and r[4] == "7.40")
            assert first[5] == "auto", first
            assert any(r[1] == "333.20" and "no MS2 signal" in r[-2] for r in t), [r for r in t if r[1] == "333.20"]   # the reason of the one with no signal, in words
            assert pg.locator("#tp-ms2map caption").count() >= 1 and pg.locator("#tp-ms2map th[scope=col]").count() > 8
            assert pg.locator(f'select[aria-label="{FIRST}"]').count() == 1
            assert pg.evaluate("document.querySelector('#tp-ms2map select').getBoundingClientRect().height") >= 24
            assert "Drift per file" in pg.inner_text("#tp-ms2map")
            ms2 = pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].map(r => [r.dataset.id, r.lastElementChild.textContent.trim()])")
            assert ["7", "1 MS2 experiment"] in ms2 and ["10007", "2 MS2 experiments"] in ms2, ms2           # the column «MS2» of the table of rows, readable as text
        step("every experiment is in the card with its state or its reason; the table of rows has the column «MS2»", card)

        def similarity():
            pg.click('#tp-t tr.clk[data-id="7"]'); pg.click('#tp-det [data-tab="identity"]'); pg.wait_for_selector("#tp-det .sim3", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('#tp-det .sim3 .bar3').length") == 6        # three components, cosine and entropy
            assert "modified cosine" in pg.inner_text("#tp-det .words"), pg.inner_text("#tp-det .words")
            pg.click('#tp-t tr.clk[data-id="0"]'); pg.wait_for_selector("#tp-det .words", timeout=60000)
            assert pg.locator("#tp-det .sim3").count() == 0, "the parent is not compared with itself"
        step("the detail of a row with MS2 shows the similarity to the parent's MS2 (the parent has none)", similarity)

        def shots(tag):
            out = os.environ.get("MZLAB_PIPE_SHOTS")
            if not out or not pg.locator("#tp-ms2map").count(): return
            for scheme in ("light", "dark"):
                for w in (1280, 390):
                    pg.emulate_media(color_scheme=scheme); pg.set_viewport_size({"width": w, "height": 1100}); lib.fermo(pg, "#tp-ms2map")
                    pg.locator("#tp-ms2map").screenshot(path=str(Path(out) / f"ms2_{tag}_{scheme}_{w}.png"))
            pg.emulate_media(color_scheme="light"); pg.set_viewport_size({"width": 1280, "height": 1000})
        shots("card")

        def move():
            pg.focus(f'select[aria-label="{FIRST}"]'); pg.select_option(f'select[aria-label="{FIRST}"]', "10007")
            pg.wait_for_function("document.querySelector('#tp-ms2-msg') && /kept/.test(document.querySelector('#tp-ms2-msg').textContent)", timeout=60000)
            t = pg.evaluate(TABLE)
            moved = next(r for r in t if r[1] == "257.06" and r[2] == "10" and r[4] == "7.40")
            assert moved[5] == "user" and moved[6].startswith("10007"), moved
            ms2 = dict(pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].map(r => [r.dataset.id, r.lastElementChild.textContent.trim()])"))
            assert ms2["7"] == "no MS2" and ms2["10007"] == "3 MS2 experiments", ms2       # the counts follow the correction
            assert pg.evaluate("document.activeElement && document.activeElement.getAttribute('aria-label')") == FIRST, "the focus stays on the menu"
        step("the menu moves an MS2 to another row (state «user», counts updated, focus kept)", move)

        def detail():
            pg.click('#tp-t tr.clk[data-id="10007"]'); pg.click('#tp-det [data-tab="identity"]'); pg.wait_for_selector(".tp-ms2-l li", timeout=60000)
            assert pg.locator(".tp-ms2-l li").count() == 3, pg.inner_text(".tp-ms2-l")
            assert pg.locator("#tp-ms2-ce option").count() == 3
            pg.select_option("#tp-ms2-ce", "40")
            fr = pg.inner_text("#tp-ms2-fr")
            assert "197.06" in fr, fr                                                        # the 60 Da loss of the CE 40 spectrum
            pg.locator(".tp-ms2-ch").first.click()
            assert pg.evaluate("document.activeElement && document.activeElement.dataset.ep") is not None, "«Change» goes to the menu"
            assert "from lr_MS2-t30" in pg.inner_text(".words"), pg.inner_text(".words")
        step("the detail of the row shows its matches, a spectrum per CE, «Change» goes to the menu", detail)

        def rep_chart():                                   # the representative scan: chart with the precursor, the sentence, the copy button, the filter
            if not pg.locator('#tp-det [data-tab="overview"]').is_visible(): pg.click('#tp-t tr.clk[data-id="10007"]')      # the row of the previous step may still be open (a second click closes it)
            pg.click('#tp-det [data-tab="overview"]'); pg.wait_for_selector("#tpr-ms2-text", timeout=60000)
            txt = pg.inner_text("#tpr-ms2-text")
            assert txt.startswith("Fragment ions found") or txt.startswith("No fragment ions found"), txt
            if txt.startswith("Fragment ions found"): assert "loss \u2212\u0394Pm/z = \u2212" in txt, txt
            lab = pg.evaluate("[...document.querySelectorAll('#tp-det svg[aria-label]')].map(s => s.getAttribute('aria-label'))")
            assert any("precursor m/z" in l and "RT" in l for l in lab), lab                  # RT and precursor m/z in the title
            assert pg.locator("#tp-det .st-prec").count() == 1, "the precursor is marked in the chart"
            assert pg.locator("#tp-det .tdbtn").count() >= 2, "the charts have their table of the data (button, or the T key on the focused chart)"
            assert "Representative MS2" in pg.inner_text("#tp-det .tpr-line"), pg.inner_text("#tp-det .tpr-line")
            assert pg.evaluate("getComputedStyle(document.getElementById('tpr-ms2-text')).userSelect") != "none"
            pg.focus("#tpr-ms2-copy"); pg.keyboard.press("Enter")
            pg.wait_for_function("/Copied|Selected/.test(document.querySelector('#tpr-ms2-text').parentElement.textContent)", timeout=10000)
            try: assert pg.evaluate("navigator.clipboard.readText()") == txt
            except Exception as e: assert "Selected" in pg.inner_text("#tpr-ms2-text + button + span"), e        # no clipboard permission here: the text is selected
        step("the representative MS2: chart with the precursor, the sentence «Fragment ions found», «Copy text», Δm/z and tolerance in words", rep_chart)

        def no_ms2():
            ids = pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].filter(r => r.lastElementChild.textContent.trim() === 'no MS2').map(r => r.dataset.id)")
            assert ids, "a row without MS2"
            pg.click(f'#tp-t tr.clk[data-id="{ids[0]}"]'); pg.click('#tp-det [data-tab="overview"]'); pg.wait_for_selector("#tp-det .tpr-none", timeout=60000)
            assert "No MS\u00b2 in the feature peak" in pg.inner_text("#tp-det"), pg.inner_text("#tp-det")[:400]
            assert pg.locator("#tpr-ms2-text").count() == 0 and pg.locator("#tp-det .st-prec").count() == 0
        step("a row with no MS2 in its peak: no chart, «No MS\u00b2 in the feature peak»", no_ms2)

        def filter_ms2():
            n_all = pg.locator("#tp-t tr.clk").count()
            pg.click('[data-chip="ms2"]'); pg.wait_for_function(f"document.querySelectorAll('#tp-t tr.clk').length < {n_all}", timeout=30000)
            cells = pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].map(r => r.lastElementChild.textContent.trim())")
            assert cells and all(not c.startswith("no MS2") for c in cells), cells                   # only rows with an MS2 scan inside the peak
            pg.click('[data-chip="ms2"]'); pg.wait_for_function(f"document.querySelectorAll('#tp-t tr.clk').length === {n_all}", timeout=30000)
        step("the filter «With MS2» keeps only the rows with an MS2 in their peak", filter_ms2)

        def reload_keeps():
            saved = pg.evaluate("localStorage.getItem('qqq.tabui.ms2links')")
            assert saved and "lr_MS2-t30.mzML" in saved, saved
            pg.reload(); pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=120000)
            tab_and_dig(); table_view()
            t = pg.evaluate(TABLE)
            assert next(r for r in t if r[1] == "257.06" and r[2] == "10" and r[4] == "7.40")[5] == "user", t
        step("the correction is kept in the browser: after a reload and a new Dig it is there (state «user»)", reload_keeps)

        def keyboard():
            pg.evaluate("(() => { const h = document.querySelector('#tp-ms2map h3'); h.tabIndex = -1; h.focus(); })()")
            pg.keyboard.press("Tab")
            ep = pg.evaluate("document.activeElement && document.activeElement.dataset.ep")
            assert ep, "Tab does not reach a menu"
            before = pg.evaluate("document.activeElement.value")
            pg.keyboard.press("ArrowDown")
            pg.wait_for_function("document.querySelector('#tp-ms2-msg') && /kept|Matching/.test(document.querySelector('#tp-ms2-msg').textContent)", timeout=60000)
            pg.wait_for_function(f"document.activeElement && document.activeElement.dataset.ep === '{ep}'", timeout=60000)
            assert pg.evaluate("document.activeElement.value") != before, "the keyboard did not change the row"
        step("keyboard: Tab reaches a menu and the arrows change the row", keyboard)

        def none_and_reset():
            pg.select_option(f'select[aria-label="{FIRST}"]', "none")
            pg.wait_for_function("document.querySelector('#tp-ms2map') && /Set to \"none\"/.test(document.querySelector('#tp-ms2map').textContent)", timeout=60000)
            ms2 = dict(pg.evaluate("[...document.querySelectorAll('#tp-t tr.clk')].map(r => [r.dataset.id, r.lastElementChild.textContent.trim()])"))
            assert ms2["7"] == "no MS2", ms2
            pg.click("#tp-ms2-reset")
            pg.wait_for_function("document.querySelector('#tp-ms2-reset') && document.querySelector('#tp-ms2-reset').disabled && !/Set to \"none\"/.test(document.querySelector('#tp-ms2map').textContent)", timeout=60000)
            t = pg.evaluate(TABLE)
            assert next(r for r in t if r[1] == "257.06" and r[2] == "10" and r[4] == "7.40")[5] == "auto", t
            assert not pg.evaluate("(JSON.parse(localStorage.getItem('qqq.tabui.ms2links') || '{}'))[Object.keys(JSON.parse(localStorage.getItem('qqq.tabui.ms2links') || '{}'))[0]]")
        step("«none» takes the scans away; «Reset matches» brings the automatic matches back and forgets the corrections", none_and_reset)

        def xlsx():
            with pg.expect_download(timeout=120000) as d:
                pg.click("#tp-x-xlsx")
            z = zipfile.ZipFile(d.value.path()); wb = z.read("xl/workbook.xml").decode()
            assert 'name="MS2 matches"' in wb and wb.index('name="MS2"') < wb.index('name="MS2 matches"'), wb[:600]
            assert any(b"Similarity to the parent" in z.read(n) for n in z.namelist() if n.endswith(".xml")), "the sheet MS2 lacks the column of the similarity"
        step("the Excel export has the sheet «MS2 matches» beside «MS2»", xlsx)

        AXE = os.environ.get("AXE_JS") or lib.axe_path()
        def axe():
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""axe.run('#tp-ms2map', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length, v.nodes.map(n => n.target.join(' ')).join(' | ').slice(0, 300)]))""")
            assert not v, v
        step("axe: the card has no violation", axe) if AXE else print("  SKIP axe-core not available (AXE_JS)")

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
            assert [u for u in reqs if "wasm-feat/finder-lr" in u], "the finder-lr .wasm was fetched"
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("no pyodide* request; MZLAB_LR_MISS empty", no_pyodide)
        shots("after")
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
