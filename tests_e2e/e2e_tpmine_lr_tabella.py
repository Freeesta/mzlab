"""Tabui, low resolution: the table of the Dig results is a TPTAB (TP_Mine/js/20_tpmine.js, 42_tp_table.js, 44_tp_riga.js). Synthetic Full Scan series (TP_Mine/tests/lr_synth.py)
opened in the Data tab and dug from Tabui: `#tp-t` is a <table> with at most 12 default columns, the parent stays first after sorting by m/z, a chip removes and restores rows
(«N of M rows» changes), a row opens under itself (XIC inside `tr.tpt-exp`), opening another closes the first, arrows move between the tabs, Esc returns the focus, a click on
a gem opens its row, no `.confetti` with reduced motion, the summary card in three blocks (Parent, Output, Download) with a help for every item, no pyodide* request, `MZLAB_LR_MISS` empty, axe clean. Builds the site and static/wasm-feat (SKIP without cargo,
wasm-bindgen or numpy); MZLAB_PIPE_SITE=<built site with static/wasm-feat> reuses one; AXE_JS=<axe.min.js>; shots: MZLAB_PIPE_SHOTS=<folder>. Run from the repo root."""
import os, shutil, subprocess, sys, tempfile, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(HERE)); import lib  # noqa: F401,E402

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

tmp = Path(tempfile.mkdtemp(prefix="tpmine_lr_tab_"))
reuse = os.environ.get("MZLAB_PIPE_SITE")
site = Path(reuse) if reuse else tmp / "site"
if not reuse:
    cmd = [sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(site)]
    subprocess.run(cmd, check=True)
    from lib_sito import add_wasm; add_wasm(site)
    from lib import build_feat; build_feat(site / "static", dict(os.environ, RUNNER_TEMP=str(tmp / "rt")))
else:                                                      # fresh sources of mzFinder over the site that was built
    for d in ("files", "js"):
        for f in (ROOT / "TP_Mine" / d).glob("*"): shutil.copy2(f, site / "static" / "mzfinder" / f.name)
series = lr_synth.write_series(tmp / "ser", times=(0, 15, 30, 60), blank=False)
EXPS = [{"precursor": 257.06, "ce": 10.0, "peaks": [(7.4, 0.07, 6e5)]},                     # the first +O isomer (RT 7.4)
        {"precursor": 241.06, "ce": 10.0, "peaks": [(9.0, 0.07, 6e5)]}]                     # the parent (RT 9.0)
ms2 = lr_synth.write_ms2(tmp / "ser", EXPS, 30.0, name="lr_MS2-t30.mzML")
paths = [str(f["path"]) for f in series] + [str(tmp / "ser" / ms2["name"])]
SMILES = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O"
PORT = lib.free_port()
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
errs = []; reqs = []
for _ in range(100):
    try: urllib.request.urlopen(f"http://127.0.0.1:{PORT}/", timeout=1).close(); break
    except Exception: srv.poll()
COUNT = "(document.querySelector('#tp-view .tpt-n')||{textContent:''}).textContent.trim()"
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        ctx = b.new_context(viewport={"width": 1280, "height": 1000}, reduced_motion="reduce")
        pg = ctx.new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.add_init_script("try{localStorage.setItem('qqq.admin','1');}catch(_){}")      # Tabui exists only in the administrator mode
        pg.on("pageerror", lambda e: errs.append(str(e)))
        ctx.on("request", lambda r: reqs.append(r.url))
        pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
        pg.wait_for_selector('#nav button[data-v="tpmine"]', timeout=60000)
        pg.set_input_files("#pick", paths)
        pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(paths)}", timeout=120000)
        pg.click("#opbtn"); pg.wait_for_selector(".pnl.chrom canvas", timeout=120000)
        pg.wait_for_function(f"E.files.length >= {len(paths)} && E.pending.length === 0", timeout=240000)

        def dig():
            pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go")
            pg.wait_for_function("document.querySelectorAll('#tp-files tr').length > 1", timeout=60000)
            try: pg.wait_for_function("document.querySelector('#tp-tol').classList.contains('tp-est')", timeout=60000)
            except Exception: pass
            pg.fill("#tp-mol", SMILES)
            pg.click("#tp-go"); pg.wait_for_selector("#tp-t", timeout=360000)
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t tr.clk", timeout=60000)
        step("the Dig of the synthetic series gives the table", dig)

        def structure():
            assert pg.evaluate("document.getElementById('tp-t').tagName") == "TABLE"
            n = pg.locator("#tp-t thead th").count()
            assert 2 <= n <= 12, n                                 # 11 data columns by default (Lvl, TP, Identity, Status, Net change, Modification, m/z, RT, Trend, Cells, Max area) + Conditions with groups
            first = pg.evaluate("document.querySelector('#tp-t tbody tr.clk').dataset.id")
            assert first == "0", "the parent is the first row: " + first
            assert pg.locator("#tp-t .tpt-pin").count() == 1
        step("#tp-t is a table with at most 12 columns, the parent first", structure)

        def sort_pin():
            pg.click("#tp-t th button.tpt-sh >> text=m/z")
            ids = pg.evaluate("[...document.querySelectorAll('#tp-t tbody tr.clk')].map(r => r.dataset.id)")
            assert ids[0] == "0", ids[:3]
            pg.click("#tp-t th button.tpt-sh >> text=m/z")
            assert pg.evaluate("document.querySelector('#tp-t tbody tr.clk').dataset.id") == "0"
        step("the parent stays first after sorting by m/z (both directions)", sort_pin)

        def chips():
            names = ["strong", "possible", "weak", "unexpected"]
            base = pg.evaluate(COUNT); changed = 0
            for k in names:
                c = pg.locator(f'#tp-view .tpt-chip[data-chip="{k}"]')
                assert c.count() == 1, k
                c.click(); after = pg.evaluate(COUNT)
                if after != base: changed += 1
                c.click(); assert pg.evaluate(COUNT) == base, (k, base, pg.evaluate(COUNT))
            assert changed >= 1, "no chip changes the count: " + base
            assert " of " in base and "rows" in base, base
        step("a chip removes and restores rows; «N of M rows» changes", chips)

        def weak_note():
            t = pg.inner_text('.hero .hc[data-k="weak"]')
            assert "weak evidence" in t, t
            assert pg.locator("#tp-nopar").count() == 0, "the right parent: no «Parent not found»"
        step("the count of the weak band is always in the summary card", weak_note)

        def expand():
            pg.click('#tp-t tr.clk[data-id="0"]')
            pg.wait_for_selector("#tp-t tr.tpt-exp #tp-det .tpr-ov svg", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('#tp-t tr.tpt-exp').length") == 1
            assert pg.evaluate("document.querySelector('#tp-t tr.tpt-exp').previousElementSibling.dataset.id") == "0"
            other = pg.evaluate("[...document.querySelectorAll('#tp-t tbody tr.clk')].map(r => r.dataset.id).find(i => i !== '0')")
            assert other is not None
            pg.click(f'#tp-t tr.clk[data-id="{other}"]'); pg.wait_for_selector("#tp-t tr.tpt-exp #tp-det", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('#tp-t tr.tpt-exp').length") == 1
            assert pg.evaluate("document.querySelector('#tp-t tr.tpt-exp').previousElementSibling.dataset.id") == other
            assert pg.evaluate("document.querySelectorAll('#tp-t [aria-expanded=true]').length") == 1
        step("a row opens under itself with the XIC; opening another closes the first", expand)

        def keys():
            pg.focus('#tp-det [role=tab][aria-selected=true]')
            pg.keyboard.press("ArrowRight")
            assert pg.evaluate("document.activeElement.getAttribute('data-tab')") == "samples", pg.evaluate("document.activeElement.outerHTML")
            pg.wait_for_selector("#tp-det .lr-sg", timeout=30000)
            pg.keyboard.press("ArrowRight"); assert pg.evaluate("document.activeElement.getAttribute('data-tab')") == "identity"
            pg.keyboard.press("ArrowLeft"); assert pg.evaluate("document.activeElement.getAttribute('data-tab')") == "samples"
            pg.keyboard.press("Escape")
            assert pg.evaluate("document.querySelectorAll('#tp-t tr.tpt-exp').length") == 0
            assert pg.evaluate("!!document.activeElement.closest('#tp-t')"), "the focus returns to the table"
        step("arrows move between the tabs; Esc closes the row and returns the focus", keys)

        def decided():
            assert pg.locator("#tp-t tbody .lvb").count() == 0, "no level proposed: the Lvl column is empty"
            other = pg.evaluate("[...document.querySelectorAll('#tp-t tbody tr.clk')].map(r => r.dataset.id).find(i => i !== '0')")
            pg.click(f'#tp-t tr.clk[data-id="{other}"]'); pg.wait_for_selector("#tp-det .tpd-sel", timeout=60000)
            sel = pg.locator("#tp-det .tpd-sel").first
            sel.select_option("2b"); lib.attendi(pg, "!!document.querySelector('#tp-det .tpd-msg').textContent.trim()")
            sel.select_option("4"); lib.attendi(pg, "document.querySelectorAll('#tp-t tbody .lvb.tpt-dec').length === 1")
            assert pg.inner_text("#tp-t tbody .lvb.tpt-dec").strip() == "L4"
            assert "no level proposed" in pg.get_attribute("#tp-t tbody .lvb.tpt-dec", "aria-label")
            sel.select_option(""); lib.attendi(pg, "document.querySelectorAll('#tp-t tbody tr:not(.tpt-exp) .lvb').length === 0")
            pg.keyboard.press("Escape")
        step("Lvl stays empty until the chemist decides; 2b is refused, L4 shows in the row", decided)

        def node():
            pg.click('.vt[data-v="map"]'); pg.wait_for_selector(".mnode")
            pg.locator(".mnode").nth(1).click()
            pg.wait_for_selector("#tp-t tr.tpt-exp #tp-det", timeout=60000)
            assert pg.evaluate("document.querySelector('.vt[data-v=tab]').getAttribute('aria-selected') || document.querySelector('.vt[data-v=tab]').classList.contains('on')")
        step("a click on a map node opens its row in the Table view", node)

        def calm():
            assert pg.locator(".confetti").count() == 0, "confetti with prefers-reduced-motion"
            assert pg.evaluate("document.querySelectorAll('#tp-cnt .hc').length") >= 1
        step("reduced motion: no confetti; the counters are words", calm)

        def summary():
            t = pg.inner_text(".hero")
            assert not any(w in t for w in ("Vein found", "nuggets", "Poor vein", "\u26cf")), t[:300]
            assert pg.locator(".hero svg").count() == 0, "no pickaxe icon"
            assert [h.strip() for h in pg.locator(".hero h2").all_inner_texts()] == ["Parent", "Output", "Download"]
            par = pg.inner_text(".hero-b:nth-child(1)")
            assert "Neutral formula" in par and "[M+H]+" in par and "m/z" in par, par
            assert pg.locator(".hero-b:nth-child(3) [role=group] button").all_inner_texts() == ["Excel (.xlsx)", "CSV", "JSON", "Copy summary"]
            bare = pg.evaluate("[...document.querySelectorAll('.hero .hc:not(.hn), .hero button')].filter(e => !e.hasAttribute('data-tt')).map(e => e.textContent)")
            assert not bare, ("every item and button has its help", bare)
            pg.locator('.hero .hc[data-k="formula"]').focus(); lib.attendi(pg, "!document.getElementById('tph-pop').hidden && document.getElementById('tph-pop').textContent.includes('Neutral formula')")
            if pg.locator("#tp-weak-s").count():
                base = pg.evaluate(COUNT); pg.click("#tp-weak-s")
                lib.attendi(pg, "document.getElementById('tp-weak-s').getAttribute('aria-pressed') === 'true'")
                assert pg.inner_text("#tp-weak-s") == "hide" and "shown" in pg.inner_text("#tp-weak-w") and pg.evaluate(COUNT) != base, pg.evaluate(COUNT)
                assert pg.get_attribute('#tp-view .tpt-chip[data-chip="weak"]', "aria-pressed") == "true", "the card and the chip of the table say the same"
                pg.click("#tp-weak-s"); lib.attendi(pg, "document.getElementById('tp-weak-s').getAttribute('aria-pressed') === 'false'")
                assert pg.evaluate(COUNT) == base
            else: print("  SKIP no row with weak evidence in this series")
            pg.set_viewport_size({"width": 360, "height": 800}); lib.attendi(pg, "innerWidth === 360")
            o = pg.evaluate("(() => { const h = document.querySelector('.hero'); return [h.scrollWidth, h.clientWidth, h.getBoundingClientRect().right, innerWidth]; })()")
            pg.set_viewport_size({"width": 1280, "height": 1000}); lib.attendi(pg, "innerWidth === 1280")
            assert o[0] <= o[1] + 1 and o[2] <= o[3] + 1, ("the card fits 360 px", o)
        step("summary card: Parent, Output, Download; no title of outcome, every item with its help, the weak band in one click", summary)

        def more():
            n = pg.evaluate("[document.querySelectorAll('details[name=tp-more]').length, document.querySelectorAll('details[name=tp-more][open]').length, document.querySelectorAll('#tp-x-xlsx, #tp-x-csv, #tp-x-json, #tp-x-txt').length, document.querySelectorAll('#tp-view .tpt-xm').length]")
            assert n[0] >= 1 and n[1] == 0 and pg.locator('#tp-m-ms2 > summary').count() == 1 and n[2] == 4 and n[3] == 1, n
        step("«More» is closed; the old exports and «Export view» are there", more)

        AXE = os.environ.get("AXE_JS") or lib.axe_path()
        def axe():
            pg.click('.vt[data-v="tab"]'); pg.wait_for_selector("#tp-t tr.clk")
            pg.add_script_tag(path=AXE)
            v = pg.evaluate("""axe.run('#tp-view', {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.map(v => [v.id, v.impact, v.nodes.length, v.nodes.map(n => n.target.join(' ')).join(' | ').slice(0, 300)]))""")
            assert not v, v
        step("axe: the table with an open row has no violation", axe) if AXE else print("  SKIP axe-core not available (AXE_JS)")

        out = os.environ.get("MZLAB_PIPE_SHOTS")
        def shots():
            if not out: return
            Path(out).mkdir(parents=True, exist_ok=True)
            for scheme in ("light", "dark"):
                for w in (1280, 390):
                    pg.emulate_media(color_scheme=scheme, reduced_motion="reduce"); pg.set_viewport_size({"width": w, "height": 1100}); lib.fermo(pg, "#tp-t")
                    pg.locator("#tp-view").screenshot(path=str(Path(out) / f"lr_tabella_{scheme}_{w}.png"))
        step("screenshots", shots)

        def no_pyodide():
            py = [u for u in reqs if "pyodide" in u.lower()]
            assert not py, py[:5]
            assert pg.evaluate("(window.MZLAB_LR_MISS || []).length") == 0, pg.evaluate("window.MZLAB_LR_MISS")
        step("no pyodide* request; MZLAB_LR_MISS empty", no_pyodide)
        b.close()
finally:
    srv.terminate()
    shutil.rmtree(tmp, ignore_errors=True)
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
