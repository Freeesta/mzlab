"""Flow, the graphical workflow page (docs/agenti/flow-grafica.md): its tab is in the header at first load, the palette comes from the three lists of
descriptors, a chain is built by clicking steps (each is wired to what fits), the plan is validated after every change and leaves and returns as mzflow/2 JSON. Also the click budget of
rule 26 (few clicks): a valid four-step chain exported in at most CEILING clicks and keys, counted by capture listeners; the five clicks of the logo are reported apart.
Run from the repo root: python3 tests_e2e/e2e_flow.py"""
import json, subprocess, sys, tempfile
from pathlib import Path
from playwright.sync_api import sync_playwright
HERE = Path(__file__).resolve().parent; ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "tools")); sys.path.insert(0, str(HERE)); import lib  # noqa: F401  (patches wait_for_function for the CSP)
from lib import free_port, attendi; PORT = free_port()
CEILING = 8          # tab + four steps + export (+ slack for one correction); a flow that needs more is a regression
ADMIN = 5            # clicks on the logo that show the tab, once per browser
COUNT = """() => { window.__n = {click: 0, key: 0}; const M = ['Shift', 'Control', 'Alt', 'Meta'];
  addEventListener('click', e => { if (e.isTrusted) window.__n.click++; }, true);
  addEventListener('keydown', e => { if (e.isTrusted && !M.includes(e.key) && !e.repeat) window.__n.key++; }, true); }"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:1500]))

tmp = Path(tempfile.mkdtemp())
subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(tmp / "site")], check=True)
from lib_sito import add_wasm; add_wasm(tmp / "site")
site = tmp / "site"
srv = subprocess.Popen([sys.executable, str(HERE / "servi.py"), str(PORT), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
import socket
for _ in range(2000):
    try: socket.create_connection(("127.0.0.1", PORT), timeout=1).close(); break
    except OSError: pass
errs = []; reqs = []
TAB = '#nav button[data-v="flow"]'
def add(pg, sid): pg.click(f'.fl-it[data-step="{sid}"]')
try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        def fresh(admin, reduced=False):
            c = b.new_context(viewport={"width": 1400, "height": 1000}, reduced_motion="reduce" if reduced else "no-preference", accept_downloads=True); pg = c.new_page()
            pg.on("pageerror", lambda e: errs.append(str(e))); pg.on("request", lambda r: reqs.append(r.url))
            if admin: pg.add_init_script("try{localStorage.setItem('mzlab.admin','1');}catch(_){}")
            pg.goto(f"http://127.0.0.1:{PORT}/"); pg.wait_for_selector("#drop", timeout=120000)
            return pg
        pg = fresh(False)
        def shown():
            pg.wait_for_selector(TAB, timeout=60000)
            assert pg.evaluate("localStorage.getItem('mzlab.admin')") is None
        step("a normal user has the Flow tab at first load", shown)
        pg.context.close()

        pg = fresh(True, reduced=True)
        pg.wait_for_selector(TAB, timeout=60000); pg.evaluate(COUNT)
        pg.click(TAB); pg.wait_for_selector("#flx-root", timeout=30000)
        # the list view is the editor under the canvas: the same plan, started empty here; the setup clicks are not part of the budget below
        pg.click('[data-tab="list"]'); pg.click('button[aria-label="Start a new plan"]'); pg.evaluate("document.querySelector('.flx-pal').remove()")
        def palette():
            assert pg.locator(".fl-it").count() == 47, pg.locator(".fl-it").count()
            assert pg.locator(".fl-pal details").count() == 3
            pg.fill("#fl-filter", "formula"); attendi(pg, "[...document.querySelectorAll('.fl-it')].filter(b => !b.hidden).length >= 2")
            assert pg.locator(".fl-it:visible").count() < 47
            pg.fill("#fl-filter", "")
        step("palette: 47 steps in three packages, the filter narrows it", palette)
        def build():
            pg.evaluate("window.__n = {click: 1, key: 0}")      # the tab click counts; the palette checks and the list-view setup before do not
            for s in ["convert.mzml", "index.mzidx", "features.detect", "features.align"]: add(pg, s)
            attendi(pg, "document.querySelectorAll('.fl-card').length === 4")
            assert "4 steps · valid" in pg.inner_text("#fl-status"), pg.inner_text("#fl-status")
            assert pg.locator(".fl-chip").count() >= 1 and pg.locator("#fl-add-input").count() == 1       # the raw input was added by the first step
        step("four clicks build a valid chain, wired by type", build)
        def export():
            with pg.expect_download() as d: pg.click('button[aria-label="Export the plan as mzflow/2 JSON"]')
            plan = json.loads(Path(d.value.path()).read_text())
            assert plan["format"] == "mzflow/2" and [s["id"] for s in plan["steps"]] == ["convert", "index", "detect", "align"], plan
            assert plan["steps"][2]["in"] == {"index": ["index.index"]}, plan["steps"][2]
            Path(tmp / "plan.json").write_text(json.dumps(plan))
            n = pg.evaluate("window.__n"); total = n["click"] + n["key"]
            print(f"MISURA clic_flow: tab 1 + 4 steps + export 1 = {n['click']} clicks + {n['key']} keys = {total}; with the {ADMIN} clicks of the logo {total + ADMIN}")
            assert total <= CEILING, n
        step("export gives mzflow/2 in the order of the run, in few clicks", export)
        def problems():
            pg.click('button[aria-label="Remove input input1.raw"]')
            attendi(pg, "document.querySelector('#fl-status').textContent.includes('problem')")
            assert "Input raw is not connected." in pg.inner_text("#fl-card-convert"), pg.inner_text("#fl-card-convert")
            assert "problem" in pg.inner_text("#fl-status") and pg.locator(".fl-card.bad").count() >= 1
        step("a missing input shows on its card and in the status", problems)
        def roundtrip():
            pg.set_input_files("#fl-file", str(tmp / "plan.json"))
            attendi(pg, "document.querySelectorAll('.fl-card').length === 4 && document.querySelector('#fl-status').textContent.includes('valid')")
            pg.set_input_files("#fl-file", {"name": "bad.json", "mimeType": "application/json", "buffer": b"{"})
            attendi(pg, "document.querySelector('#fl-msg').textContent.includes('not valid JSON')")
        step("the exported plan opens again; a broken file says why", roundtrip)
        def copy_command():
            pg.click('button[aria-label="Export and run with the command line"]')
            attendi(pg, "(document.querySelector('.fl-cli code') || {textContent: ''}).textContent.includes('mzlab-cli flow run')")
            text = pg.inner_text(".fl-cli code")
            assert "<<'MZFLOW'" in text and '--in "$MZLAB_IN"' in text, text[:300]
            # the here-doc is the plan itself: the shell rewrites the same bytes the export gave
            body = text.split("<<'MZFLOW'\n", 1)[1].split("\nMZFLOW\n", 1)[0]
            with pg.expect_download() as d: pg.click('button[aria-label="Export the plan as mzflow/2 JSON"]')
            got = json.loads(Path(d.value.path()).read_text())      # engine: scalar references and no empty sha256; page export: lists and ""; the same plan
            norm = lambda p: {**p, "inputs": [{k: v for k, v in i.items() if not (k == "sha256" and v == "")} for i in p["inputs"]], "steps": [{**x, "in": {k: v if isinstance(v, list) else [v] for k, v in x.get("in", {}).items()}} for x in p["steps"]]}; b = json.loads(body); assert norm(b) == norm(got), {k: (b.get(k), got.get(k)) for k in set(b) | set(got) if b.get(k) != got.get(k)}
        step("Copy command shows the shell text built by the engine, with the plan inside", copy_command)
        def parameters():
            pg.fill("#fl-p-detect-floor", "5000"); pg.keyboard.press("Tab")
            attendi(pg, "!!document.querySelector('#fl-card-detect button[aria-label^=\"Back to the default\"]')")
            with pg.expect_download() as d: pg.click('button[aria-label="Export the plan as mzflow/2 JSON"]')
            plan = json.loads(Path(d.value.path()).read_text())
            assert plan["steps"][2]["params"] == {"floor": 5000}, plan["steps"][2]
            pg.click('#fl-card-detect button[aria-label^="Back to the default"]')
            attendi(pg, "!document.querySelector('#fl-card-detect button[aria-label^=\"Back to the default\"]')")
            pg.select_option("#fl-alg-detect", "threshold"); attendi(pg, "document.querySelector('#fl-card-detect').textContent.includes('is planned, not available yet')")
            pg.select_option("#fl-alg-detect", "masstrace")
        step("the form: only differences from the default are written; a planned algorithm is refused", parameters)
        def goal():
            attendi(pg, "document.querySelector('.fl-goal').textContent.includes('does not reach the identification level yet. Next:')")      # «Draw a structure» needs no input: the level is always reachable
            pg.select_option("#fl-add-input", "spectra"); pg.select_option("#fl-add-input", "library")
            attendi(pg, "document.querySelector('.fl-goal').textContent.includes('does not reach the identification level yet')")
        step("the goal: the page says the chain does not reach the identification level and what comes next", goal)
        def keyboard_and_motion():
            pg.focus('.fl-it[data-step="table.filter"]'); pg.keyboard.press("Enter")
            attendi(pg, "document.querySelectorAll('.fl-card').length === 5")
            pg.click('button[aria-label="Move filter up"]')
            attendi(pg, "document.querySelectorAll('.fl-card')[3].dataset.id === 'filter'")
            assert pg.evaluate("getComputedStyle(document.querySelector('.fl-card')).animationName") == "none"
            pg.click('button[aria-label="Remove filter"]'); attendi(pg, "document.querySelectorAll('.fl-card').length === 4")
        step("keyboard adds a step, buttons reorder and remove it, reduced motion stops the animation", keyboard_and_motion)
        def compare():
            pg.evaluate("document.querySelector('details[data-pkg=attributio]').open = true")
            add(pg, "spectra.match")
            attendi(pg, "!!document.querySelector('#fl-card-match')")
            assert pg.locator('button[aria-label="Compare algorithms of align"]').count() == 0 and pg.locator('button[aria-label="Compare algorithms of detect"]').count() == 1     # one available algorithm only / two
            pg.click('button[aria-label="Compare algorithms of match"]')
            attendi(pg, "!!document.querySelector('#fl-card-compare')")
            assert pg.locator("#fl-card-compare .fl-var").count() == 2
            assert "Compares spectra.match@1" in pg.inner_text("#fl-card-compare")
            pg.select_option("#fl-v-compare-1-alg", pg.evaluate("document.querySelector('#fl-v-compare-0-alg').value"))
            attendi(pg, "document.querySelector('#fl-card-compare').textContent.includes('Two variants of the comparison are identical.')")
            pg.fill("#fl-p-compare-mz_tolerance_ppm", "7")      # the click on Export is the blur that commits it
            with pg.expect_download() as d: pg.click('button[aria-label="Export the plan as mzflow/2 JSON"]')
            plan = json.loads(Path(d.value.path()).read_text())
            c = [x for x in plan["steps"] if x["step"] == "meta.compare@1"][0]
            assert c["params"]["step"] == "spectra.match@1" and c["params"]["mz_tolerance_ppm"] == 7 and set(c["in"]) == set(next(x for x in plan["steps"] if x["id"] == "match")["in"]), c
        step("a comparison of algorithms is offered where two are available, checked on the page and exported", compare)
        def cli():
            attendi(pg, "document.querySelector('.fl-cli code') && document.querySelector('.fl-cli code').textContent.includes('mzlab-cli flow run')")
        step("«Run with the CLI» shows the command of the native runner", cli)

        pg = fresh(True, reduced=True)
        pg.wait_for_selector(TAB, timeout=60000); pg.evaluate(COUNT)
        pg.click(TAB); pg.wait_for_selector(".fln", timeout=30000)
        def first_screen():
            attendi(pg, "document.querySelectorAll('.fln').length === 2")
            assert "Import data" in pg.inner_text(".fln") and pg.locator(".fl-lines path").count() == 1
            assert pg.locator(".fln .fln-chip").count() == 2 and pg.locator(".fln .fln-chip.warn").count() == 1 and pg.locator(".flx-tpl.show").count() == 1 and pg.locator(".flx-tc").count() == 9 and pg.locator(".flx-tpl > .flx-tpls > .flx-tc").count() == 4 and pg.locator(".flx-more .flx-tc").count() == 5 and pg.locator(".flx-more[open]").count() == 0 and pg.inner_text(".flx-more summary") == "More templates" and pg.inner_text(".fl-insp h3") == "Build feature table"
            assert pg.get_attribute('[data-mode="simple"]', "aria-pressed") == "true"
            assert pg.locator(".flx-pal").is_hidden()
        step("canvas: the base is already there and connected, templates are offered, Simple is the default", first_screen)
        def file_list_untouched():
            # the stylesheet of Flow is now in the page: a file-list row (class «fl», the same bare class Flow once used as its root) must keep its own layout, dark theme included
            pg.evaluate("document.documentElement.setAttribute('data-theme','dark')")
            got = pg.evaluate("""() => { const p = document.createElement('div'); p.className = 'fl'; p.style.cssText = 'position:absolute;left:0;top:0;width:300px';
              p.innerHTML = '<span>box</span><span>name</span>'; document.body.append(p); const c = getComputedStyle(p), r = [c.display, c.paddingLeft, c.gridTemplateColumns, p.getBoundingClientRect().height]; p.remove(); return r; }""")
            assert got[0] != "grid" and got[1] == "0px" and got[3] <= 56, got
        step("the stylesheet of Flow leaves the rows of the file list alone (class «fl»)", file_list_untouched)
        def template():
            ids = pg.evaluate("[...document.querySelectorAll('.flx-tc')].map(b => b.dataset.tpl)")
            assert ids == ["tp", "table", "identify", "msn", "bentazone", "targeted", "tpseries", "standards", "blankfree"], ids; pg.click(".flx-more summary"); assert pg.locator(".flx-more .flx-tc").first.is_visible()
            assert pg.evaluate("[...document.querySelectorAll('.flx-tc small:not(.flx-help)')].every(s => s.textContent.startsWith('Good for:'))")
            pg.click('.flx-tc[data-tpl="identify"]')
            attendi(pg, "document.querySelectorAll('.fln').length === 7")
            assert "Ready" in pg.inner_text("#flx-status"), pg.inner_text("#flx-status")
            n = pg.evaluate("window.__n"); total = n["click"] + n["key"]
            print(f"MISURA clic_canvas: tab 1 + template 1 = {total} clicks to a complete valid chain; with the {ADMIN} clicks of the logo {total + ADMIN}")
            print(f"MISURA clic_modello: {total} clicks from the empty sheet to a ready plan (3 templates on the first row, 5 under More templates)")
            assert total <= 3, n
        step("canvas: one template click gives a complete valid chain", template)
        def inspector():
            pg.click('.fln:has-text("Match to library")')
            attendi(pg, "document.querySelector('.fl-insp h3') && document.querySelector('.fl-insp h3').textContent === 'Match to library'")
            assert "What this step does" not in pg.inner_text(".fl-insp") and "Ready." not in pg.inner_text(".fl-insp") and pg.locator(".fl-insp button:has-text('Remove step')").count() == 1
            pg.click('.fln:has-text("Import data")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Import data'")
            assert pg.locator(".fl-insp button:has-text('Remove')").count() == 0 and "cannot remove" not in pg.inner_text(".fl-insp") and "part of the base" not in pg.inner_text(".fl-insp")
        step("canvas: the inspector explains the step; the base cannot be removed", inspector)
        def needs_and_fix():
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Regulatory lists")')
            attendi(pg, "document.querySelectorAll('.fln').length === 8")
            pg.click('.fln:has-text("Regulatory lists")')
            assert pg.locator(".fln.sel .fln-chip.ok").count() == 1
            pg.click('.fl-insp button:has-text("Remove step")'); attendi(pg, "document.querySelectorAll('.fln').length === 7")
            assert "Removed" in pg.inner_text("#flx-live")
            pg.click('#flx-live button:has-text("Undo")'); attendi(pg, "document.querySelectorAll('.fln').length === 8")
            pg.click('button[aria-label="Redo the change you undid"]'); attendi(pg, "document.querySelectorAll('.fln').length === 7")
            pg.click('button[aria-label="Undo the last change"]'); attendi(pg, "document.querySelectorAll('.fln').length === 8")
            pg.keyboard.press("Control+z"); attendi(pg, "document.querySelectorAll('.fln').length === 7")
            pg.keyboard.press("Control+y"); attendi(pg, "document.querySelectorAll('.fln').length === 8 && !document.querySelector('button[aria-label=\"Redo the change you undid\"]').disabled")
            pg.click('button[aria-label="Start from a template"]'); pg.click('.flx-tc[data-tpl="table"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Confidence level")'); attendi(pg, "document.querySelectorAll('.fln').length === 3")
            pg.click('.fln:has-text("Confidence level")')
            assert "Needs input" in pg.inner_text(".fln.sel") and pg.locator(".fl-needs li").count() == 2, pg.inner_text(".fl-insp")
            pg.click('.fl-fix:has-text("Molecular formulas")'); attendi(pg, "document.querySelectorAll('.fl-needs li').length === 1")
            assert "Remove" in pg.inner_text(".fl-insp")
        step("canvas: a missing input is said on the card with a one-click fix; removing can be undone", needs_and_fix)
        def issues_and_hints():
            # after the previous step «Confidence level» still needs the library match: Issues lists it with its remedy; an estimated value says how and can be fixed; each step of the base has a «?» to the Theory
            attendi(pg, "document.querySelectorAll('.flx-issues li').length === 1")
            assert "Issues (1)" in pg.inner_text(".flx-issues h3")
            pg.click('.flx-issues li button:has-text("Show")'); attendi(pg, "document.querySelector('.fln.sel')")
            pg.click('.flx-issues button:has-text("Add “Match to library”")'); attendi(pg, "document.querySelectorAll('.flx-issues li').length === 0")
            assert pg.locator(".flx-issues").is_hidden(), "no needs: the Issues box is not shown"
            pg.click('.fln:has-text("Build feature table")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Build feature table'")
            q = pg.locator(".fl-insp-hd a.flx-q"); assert q.count() == 1 and q.get_attribute("href").startswith("teoria/21-glossario.html#g-"), q.count()
            for m in pg.locator('.fl-insp summary:has-text("More")').all(): m.click()
            assert pg.locator('.fl-insp .flx-est').count() >= 1, pg.inner_text(".fl-insp")[:400]
            pg.locator('.fl-insp .flx-est button:has-text("Use fixed")').first.click(); attendi(pg, "FLOW.current.plan.steps.some(s => s.params && Object.values(s.params).length)")
        step("canvas: Issues lists the needs with their remedy; an estimated value says how and can be fixed; «?» opens the Theory", issues_and_hints)
        def advanced_and_keys():
            pg.click('[data-mode="advanced"]'); attendi(pg, "!document.querySelector('.flx-pal').offsetParent === false")
            assert pg.locator(".flx-pal").is_visible() and pg.locator(".fln").count() > 3
            pg.click('[data-mode="simple"]')
            pg.focus('.fln:has-text("Import data")'); pg.keyboard.press("Enter")
            attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Import data'")
            pg.focus("#fl-canvas"); before = pg.evaluate("document.querySelector('.fl-stage').style.transform")
            pg.keyboard.press("ArrowLeft"); assert pg.evaluate("document.querySelector('.fl-stage').style.transform") != before
            assert pg.evaluate("getComputedStyle(document.querySelector('.fln')).animationName") == "none"
        def menus_and_redraw():
            # a redraw after an edit does not replay the entrance of the cards that were already there
            pg.focus('.fln:has-text("Import data")'); pg.keyboard.press("Enter")
            attendi(pg, "document.querySelectorAll('.fln').length > 0")
            assert pg.locator(".fln.fl-new").count() == 0
            assert "Process" in pg.get_attribute(".fln >> nth=1", "aria-label") or "Data in" in pg.get_attribute(".fln >> nth=0", "aria-label")
            pg.click("#flx-add"); assert pg.get_attribute("#flx-add", "aria-haspopup") == "menu" and pg.get_attribute("#flx-add", "aria-expanded") == "true"
            pg.keyboard.press("Escape"); assert pg.get_attribute("#flx-add", "aria-expanded") == "false"
            assert pg.get_attribute(".flx-tpl", "role") == "dialog"
        step("canvas: menus say they are open, the template card is a dialog, a redraw does not replay the entrance", menus_and_redraw)
        step("canvas: Advanced shows every step and the palette; Enter selects, arrows move the view, reduced motion stops the animation", advanced_and_keys)
        def sheets():
            pg.click("#flx-more"); assert pg.locator('[role=menuitem]:has-text("Plan (JSON)")').count() == 0; pg.keyboard.press("Escape")
            pg.click('[data-mode="advanced"]'); pg.click("#flx-more"); pg.click('[role=menuitem]:has-text("Plan (JSON)")'); assert '"format": "mzflow/2"' in pg.inner_text(".flx-plan")
            pg.click('[data-tab="list"]'); attendi(pg, "document.querySelectorAll('.fl-card').length >= 3"); pg.click('[data-tab="canvas"]'); pg.click('[data-mode="simple"]')
        step("canvas: the plan view and the list view show the same plan", sheets)
        def results_from_disk():
            pg.click('.fln:has-text("Build feature table")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Build feature table'")
            pg.set_input_files('.fl-insp input[type=file][accept]', files=[{"name": "peaks.csv", "mimeType": "text/csv", "buffer": b'mz,rt,area\n241.3,3.0,"1,5"\n257.3,4.2,7\n'}])
            attendi(pg, "document.querySelectorAll('.flx-res-body .flx-tablewrap')[1].querySelectorAll('tbody tr').length === 2")
            data = '.flx-res-body > .flx-tablewrap:last-child .flx-table'
            assert pg.locator(data + " th").all_inner_texts() == ["mz", "rt", "area"] and pg.locator(data + " td").nth(2).inner_text() == "1,5"
            # the column summary: one row per column, numbers counted from the whole file
            pg.click('.flx-qc summary'); qc = '.flx-qc .flx-table tbody tr'
            got = pg.locator(qc).nth(0).inner_text().split(); assert pg.locator(qc).count() == 3 and got == ["mz", "2", "0", "241.3", "257.3"], got
            assert pg.locator(".flx-res").get_attribute("aria-label") == "Results" and pg.evaluate("document.activeElement.closest('.flx-res') !== null")
            pg.click('.flx-res button[aria-label="Close the results"]'); assert pg.locator(".flx-res").count() == 0
        step("results: a table chosen from disk opens under the canvas as a real table, readable by keyboard and closable", results_from_disk)
        def identification_views():
            # JSON results of the identification steps become tables; an MGF file gets a list, a stick chart and its peak table
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Match to a spectral library")'); pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Confidence level")')
            pg.click('.fln:has-text("Match to library")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Match to library'")
            hits = b'{"format":"candidates/1","hits":[{"query":"q1","name":"Caffeine","formula":"C8H10N4O2","score":0.9123,"shared":5,"mass_ppm":1.2,"source":"lib","algorithm":"cosine"}]}'
            pg.set_input_files('.fl-insp input[type=file][accept]', files=[{"name": "hits.json", "mimeType": "application/json", "buffer": hits}])
            attendi(pg, "document.querySelectorAll('.flx-res-body .flx-table tbody tr').length === 1")
            assert pg.locator(".flx-res-body .flx-table td").nth(1).inner_text() == "Caffeine" and pg.locator(".flx-res-body .flx-table td").nth(3).inner_text() == "0.9123"
            pg.click('.flx-res button[aria-label="Close the results"]')
            pg.click('.fln:has-text("Confidence level")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Confidence level'")
            lv = b'{"format":"annot/1","rows":[{"id":"q1","level":"3a","why":"provvisorio","name":"Caffeine","gates":[{"id":"score","ok":false,"value":0.5,"threshold":0.7}]}]}'
            pg.set_input_files('.fl-insp input[type=file][accept]', files=[{"name": "level.json", "mimeType": "application/json", "buffer": lv}])
            attendi(pg, "document.querySelectorAll('.flx-res-body .flx-table tbody tr').length === 1")
            assert "score 0.5 / 0.7 (not met)" in pg.inner_text(".flx-res-body .flx-table tbody")
            pg.click('.flx-res button[aria-label="Close the results"]')
            pg.click('.fln:has-text("Confidence level")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Confidence level'")
            # the campaign folder is offered next to the file picker (the cascade itself needs the attributio module)
            assert pg.locator('.fl-insp input[type=file][webkitdirectory]').count() == 1 and pg.locator('.fl-insp button:has-text("Open a campaign folder")').count() == 1
        step("results: candidate and level JSON files open as tables", identification_views)
        def review_view():
            # the engine is replaced by a stub: the Review screen is driven by keyboard through two features, one saved without a note, one that asks for it
            pg.evaluate("""() => {
              const rev = (id, level) => ({ pol: "POS", id, mz: 200.1, rt: 3.2, current: { level, sub: "", name: "x", group: "" }, alternatives: [{ name: "Caffeine", score: 0.91 }], rtlogp: { ok: true },
                proposal: { class: "2b-check", proposed: level === "3" ? "2b" : "3", reasons: ["one reason"], evidence: { n_peaks: 5, n_explained: 3, frac_explained: 0.6, view: { peaks: [{ mz: 138.07, rel: 100, frag: { formula: "C6H8N3", ppm: 1.2, loss: "CO" } }] } } } });
              window.__saved = [];
              ATTRIBUTIO_CORE.pool = async () => ({ attributioExport: async (f, r, k) => ({ files: k === 'evidence' ? [{ name: 'evidence_demo.json', text: '{"settings_hash":"0123456789ab","settings":"tol=0.5","features":2}' }] : [{ name: 'out_' + k + '.csv', text: 'a;b' }], warnings: [] }), attributioReview: async (f, r, pol, id) => rev(id, id === 1 ? "2" : "3"), attributioDecision: async (f, r, d) => { window.__saved.push(JSON.parse(d)); return { summary: { path: "decisions/feature_decisions.csv" }, text: "csv" }; } });
              const data = { columns: [], rows: [["POS", 1, 200.1, 3.2, 2, "b", "b", "x", "", ""], ["POS", 2, 201.1, 3.4, 3, "a", "b", "y", "", ""]] };
              FLOW.openResult({ type: "annot", label: "demo (campaign folder)", campaign: { root: "demo", files: [], data, summary: {} }, text: async () => "" });
            }""")
            pg.click('.flx-res button:has-text("Settings digest")'); attendi(pg, "document.querySelector('.flw-digest code') && document.querySelector('.flw-digest code').textContent === '0123456789ab'")
            assert pg.locator('.flx-res .flw-digest button:has-text("Copy")').count() == 1
            # the engine exports of Attributio hang on the same campaign: six items, one file downloads
            assert pg.locator('.flx-res #at-export:not([disabled])').count() == 1 and pg.locator('.flx-res #at-export-menu button').count() == 6
            assert pg.locator('.flx-res .flw-ms2 #ms2-insilico').count() == 1 and pg.locator('.flx-res .flw-ms2 #ms2-predict').count() == 1  # the MS2 panel
            pg.click('.flx-res #at-export')
            with pg.expect_download() as dl: pg.click('.flx-res #at-export-menu button[data-kind=schymanski]')
            assert dl.value.suggested_filename == "out_schymanski.csv"
            pg.click('.flx-res button:has-text("Review")'); attendi(pg, "document.querySelector('.flw-rv h3') && /POS 1/.test(document.querySelector('.flw-rv h3').textContent)")
            assert "3 of 5 peaks explained" in pg.inner_text(".flw-rv-proofs") and pg.locator(".flw-rv-acts button").count() == 5
            pg.fill('.flw-rv input[aria-label="Reviewer"]', "ab")
            pg.focus("#flw-rv-h"); pg.keyboard.press("a")  # level 2 -> 3 changes the level: the note is asked, nothing is saved
            attendi(pg, "!document.querySelector('.flw-rv-note').hidden"); assert pg.evaluate("window.__saved.length") == 0
            pg.fill('.flw-rv-note', "seen the fragments"); pg.click('.flw-rv-acts button:has-text("Accept")')
            attendi(pg, "window.__saved.length === 1 && /POS 2/.test(document.querySelector('.flw-rv h3').textContent)")
            got = pg.evaluate("window.__saved[0]"); assert got["level"] == "3" and got["decided_by"] == "ab" and got["justification"] == "seen the fragments", got
            pg.keyboard.press("a")  # proposed 2b on a level 3 row: a note is asked again
            attendi(pg, "!document.querySelector('.flw-rv-note').hidden")
            pg.click('.flw-rv-acts button:has-text("Skip")'); assert pg.locator(".flw-rv progress").get_attribute("value") == "1"
            pg.click('.flw-rv button:has-text("Download decisions")')
        def feat_view():
            # a .mzfeat chosen from disk: the page reads its meta, opens the session of the table and mounts the table (both engines are replaced by stubs, as the test site has no table .wasm)
            pg.evaluate("""() => {
              window.PIPEQC = window.PIPEQC || {}; window.PIPET = window.PIPET || {};
              PIPEQC.open = async a => { window.__fa = a; return {}; }; PIPET.mount = (box, o) => { window.__fo = o; box.textContent = "mounted"; };
              const meta = new TextEncoder().encode(JSON.stringify({ rows: 3, files: 2, inputs: [{ name: "a.mzidx" }, { name: "b.mzidx" }], arrays: [] }));
              const head = new Uint8Array(16); head.set(new TextEncoder().encode("MZFEAT01")); new DataView(head.buffer).setBigUint64(8, BigInt(meta.length), true);
              FLOW.openResult({ type: "table", label: "demo.mzfeat", file: new File([head, meta], "demo.mzfeat"), text: async () => "" });
            }""")
            attendi(pg, "window.__fo && /3 rows in 2 files/.test(document.querySelector('.flx-res-body').textContent)")
            assert pg.evaluate("window.__fa.names.join()") == "a,b" and pg.evaluate("window.__fa.pol") == 1
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("results: a .mzfeat from disk opens the feature table with the names of its files", feat_view)
        def end_of_run():
            # the run ended with a .mzfeat made: the Feature table opens by itself (0 clicks) with the roles of the sample list of the plan and «Not in blank» on
            pg.evaluate("""async () => {
              PIPET.mount = (box, o) => { window.__fo = o; box.innerHTML = '<button data-fc="blank" aria-pressed="false">Not in blank</button>'; box.firstChild.onclick = e => e.target.setAttribute("aria-pressed", "true"); };
              const S = FLOW.current, inp = FLOW.addInput(S.plan, "samples");
              S.samples = { [inp.name]: { schema: "campioni/2", campioni: [{ file: "a.mzML", tipo: "blank", tempo_min: null }, { file: "b.mzML", tipo: "sample", tempo_min: 5 }] } };
              const meta = new TextEncoder().encode(JSON.stringify({ rows: 3, files: 2, inputs: [{ name: "a.mzidx" }, { name: "b.mzidx" }], arrays: [] }));
              const head = new Uint8Array(16); head.set(new TextEncoder().encode("MZFEAT01")); new DataView(head.buffer).setBigUint64(8, BigInt(meta.length), true);
              S.results = { s1: [{ type: "table", label: "run.mzfeat", text: async () => "", load: async () => new File([head, meta], "run.mzfeat") }] };
              window.__fo = null; window.__fa = null; await FLOW.afterRun(S);
            }""")
            attendi(pg, "window.__fo && /3 rows in 2 files/.test(document.querySelector('.flx-res-body').textContent)")
            assert pg.evaluate("window.__fa.campioni.map(c => c.tipo).join()") == "blank,sample", pg.evaluate("window.__fa.campioni")
            assert pg.evaluate("document.querySelector('.flx-res [data-fc=blank]').getAttribute('aria-pressed')") == "true"
            assert pg.locator(".flx-res .flx-leg .flx-auto").count() == 1
            print("MISURA clic_risultato: 0 clicks from the end of the run to the Feature table")
            pg.evaluate("""async () => { const S = FLOW.current; S.samples = {}; window.__fo = null; await FLOW.afterRun(S); }""")
            attendi(pg, "window.__fo && /does not|no content yet/.test(document.querySelector('.flx-res-body').textContent)")
            assert pg.evaluate("window.__fa.campioni") is None
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("a finished run opens the Feature table by itself, with the sample roles of the plan", end_of_run)
        def samples_in_place():   # the sample list is filled in place: «From Data» takes the open files, «Edit» changes a row, the input keeps the SHA-256 of the canonical document
            pg.evaluate("""() => { const S = FLOW.current; S.samples = {}; S.plan.inputs = S.plan.inputs.filter(i => i.type !== 'samples'); FLOW.addInput(S.plan, 'samples'); S.redraw(); }""")
            pg.click('.fln:has-text("Sample list")'); attendi(pg, "document.querySelector('.fl-insp .mg-sc')")
            assert pg.locator('.fl-insp button[aria-label^="Take"]').is_disabled(), "From Data needs open files"
            pg.evaluate("""() => { window.__e = E.files; E.files = [{ file: 'blank.mzML', role: 'blank', type: 'blank' }, { file: 'x_t5.mzML', role: 'sample', time: 5, type: 'sample' }]; FLOW.current.redraw(); }""")
            pg.click('.fln:has-text("Sample list")'); attendi(pg, "document.querySelector('.fl-insp .mg-sc') && !document.querySelector('.fl-insp button[aria-label^=Take]').disabled")
            pg.click('.fl-insp button[aria-label^="Take"]')
            attendi(pg, "document.querySelector('.fl-insp .mg-sc').getAttribute('aria-rowcount') === '3' && /^[0-9a-f]{12}/.test(document.querySelector('.fl-insp code').textContent)")
            sha = pg.evaluate("FLOW.current.plan.inputs.find(i => i.type === 'samples').sha256"); assert len(sha) == 64, sha
            c = pg.evaluate("+document.querySelector('.fl-insp [role=columnheader]:has(button[aria-label=\\'Sort by Time\\'])').getAttribute('aria-colindex') - 1")
            pg.click(f".fl-insp .mg-sc [data-r='1'][data-c='{c}']"); pg.keyboard.press("Enter"); pg.keyboard.press("Control+a"); pg.keyboard.type("10"); pg.keyboard.press("Tab")
            attendi(pg, "FLOW.current.plan.inputs.find(i => i.type === 'samples').sha256 !== '%s'" % sha)
            assert pg.evaluate("Object.values(FLOW.current.samples)[0].campioni[1].tempo_min") == 10
            assert "No t0" in pg.inner_text('.fl-insp'); pg.evaluate("() => { E.files = window.__e; }")
        step("the sample list is filled from Data or edited in place, and the input keeps its SHA-256", samples_in_place)

        def samples_fix():
            # a step that asks for the sample list: the one-click remedy adds it filled from the files open in Data (MISURA clic_campioni)
            pg.evaluate("""() => { const S = FLOW.current; S.samples = {}; S.plan.inputs = S.plan.inputs.filter(i => i.type !== 'samples'); window.__e = E.files; E.files = [{ file: 'blank.mzML', role: 'blank', type: 'blank' }, { file: 'x_t5.mzML', role: 'sample', time: 5, type: 'sample' }];
              FLOW.addStep(S.plan, S.reg, 'table.filter'); for (const i of S.plan.inputs.filter(i => i.type === 'samples')) FLOW.removeInput(S.plan, i.name); S.redraw(); }""")   # no step asks for the list yet (the port is optional), so the remedy is built from the problem the checker would give
            label = pg.evaluate("""() => { const S = FLOW.current, inst = S.plan.steps.find(s => s.step.startsWith('table.filter')); window.__fix = FLOW.friendly(S, { key: 'err.flow.port.missing', instance: inst.id, port: 'samples' }).fix; return window.__fix.label; }""")
            assert label == "Add sample list from Data", label
            pg.evaluate("() => window.__fix.run()")
            attendi(pg, "FLOW.current.plan.inputs.some(i => i.type === 'samples' && i.sha256.length === 64) && Object.values(FLOW.current.samples)[0].campioni.length === 2")
            print("MISURA clic_campioni: 1 click (the remedy) from a step that asks for the list to the list filled from Data")
            pg.evaluate("() => { E.files = window.__e; }")
        step("the remedy of a step that needs the sample list fills it from Data in one click", samples_fix)

        def selection():
            # one selection: a click on a row of a table is seen by the other listeners, the row gets an outline and aria-current
            pg.evaluate("""() => { const host = document.createElement('div'); host.id = 'selhost'; host.innerHTML = '<table><tbody><tr data-id="3"><td>a</td></tr><tr data-id="7"><td>b</td></tr></tbody></table>'; document.querySelector('#flx-root').append(host); window.__seen = []; FLOW.onSelect((id, from) => window.__seen.push([id, from])); FLOW.watchRows(host, 'test'); }""")
            pg.click('#selhost tr[data-id="7"] td')
            assert pg.evaluate("window.__seen") == [["7", "test"]], pg.evaluate("window.__seen")
            assert pg.evaluate("document.querySelector('#selhost tr[data-id=\"7\"]').getAttribute('aria-current')") == "true"
            pg.evaluate("() => FLOW.select('3', 'other')")
            assert pg.evaluate("document.querySelector('#selhost tr[data-id=\"3\"]').classList.contains('flx-sel')") and not pg.evaluate("document.querySelector('#selhost tr[data-id=\"7\"]').classList.contains('flx-sel')")
            pg.evaluate("() => { FLOW.select(null); document.querySelector('#selhost').remove(); }")
        step("one selection: a row clicked in a table is marked in the others with an outline", selection)
        def family_hooks():
            # the Feature table gets a column «Family» (read once per row as it comes into view) and a tab with the ions of the family
            pg.evaluate("""async () => {
              window.__redraws = 0;
              PIPEQC.open = async a => ({ row: async id => (id === 3 ? { family: { size: 2, members: [{ mz: 301.1, rt: 5.0, role: "M" }, { mz: 323.09, rt: 5.0, role: "adduct" }] } } : { family: null }) });
              PIPET.mount = (box, o) => { window.__fo = o; box.textContent = "mounted"; return { redraw: () => { window.__redraws++; } }; };
              const meta = new TextEncoder().encode(JSON.stringify({ rows: 3, files: 2, inputs: [{ name: "a.mzidx" }, { name: "b.mzidx" }], arrays: [] }));
              const head = new Uint8Array(16); head.set(new TextEncoder().encode("MZFEAT01")); new DataView(head.buffer).setBigUint64(8, BigInt(meta.length), true);
              window.__fo = null; await FLOW.openResult({ type: "table", label: "fam.mzfeat", file: new File([head, meta], "fam.mzfeat"), text: async () => "" });
            }""")
            attendi(pg, "window.__fo && window.__fo.extraCols && window.__fo.extraTabs")
            assert pg.evaluate("__fo.extraCols[0].cell({ id: 3 })") == "", "unknown until read"
            attendi(pg, "window.__redraws >= 1 && __fo.extraCols[0].cell({ id: 3 }) === '2'")
            assert pg.evaluate("__fo.extraCols[0].sort({ id: 77 })") == -1, "a row not read yet sorts first"
            pg.evaluate("() => { window.__panel = document.createElement('div'); document.body.append(window.__panel); __fo.extraTabs[0].fill(window.__panel, { id: 3 }); }")
            attendi(pg, "window.__panel.querySelectorAll('tbody tr').length === 2")
            assert "reference ion is first" in pg.evaluate("window.__panel.textContent")
            pg.evaluate("() => { window.__panel.textContent = ''; __fo.extraTabs[0].fill(window.__panel, { id: 9 }); }")
            attendi(pg, "/one row per ion/.test(window.__panel.textContent)")
            pg.evaluate("() => window.__panel.remove()")
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("the Feature table has the column and the tab «Family»", family_hooks)

        def report_and_network():
            # a run report (corsa/1) opens in the Run report view; a product network (rete/1) opens with the network view and shares the one selection
            pg.evaluate("""async () => {
              const corsa = { schema: "corsa/1", engine: { name: "mzLab", version: "1.0", platform: "native" }, inputs: [], lists: [], steps: [], total_ms: 0, levels: { files: 0, rows: 0, auto: {}, other: {} } };
              await FLOW.openResult({ type: "report", label: "corsa.json", text: async () => JSON.stringify(corsa) });
            }""")
            try: attendi(pg, "document.querySelector('.flx-res .rf h1') && /Flow run report/.test(document.querySelector('.flx-res .rf h1').textContent)", 15000)
            except Exception: raise AssertionError("report view: " + pg.inner_text(".flx-res-body")[:300] + " base=" + str(pg.evaluate("String(QTOOLS.ctx.base)")))
            pg.evaluate("""async () => { await FLOW.openResult({ type: "report", label: "other.json", text: async () => "{}" }); }""")
            attendi(pg, "/not a run report/.test(document.querySelector('.flx-res-body').textContent)")
            fx = (ROOT / "TP_Mine" / "tests" / "rete_fixture.json").read_text(encoding="utf-8")
            pg.evaluate("""async (fx) => { window.__seen = []; await FLOW.openResult({ type: "transform", label: "network.json", text: async () => fx }); FLOW.onSelect((id, from) => window.__seen.push([id, from])); }""", fx)
            attendi(pg, "document.querySelector('.flx-res .rv svg')")
            first = pg.evaluate("document.querySelector('.flx-res .rv g.nd, .flx-res .rv [data-node]').getAttribute('data-node')")
            pg.click('.flx-res .rv [data-node]:not([data-node="0"])')
            attendi(pg, "window.__seen.length >= 1 && window.__seen[window.__seen.length - 1][1] === 'rete'")
            pg.evaluate("() => FLOW.select(null, 'other')")
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("a run report and a product network open in their views and the network joins the one selection", report_and_network)

        step("results: the Review screen asks for a note only when the level changes and moves to the next feature", review_view)
        def mgf_view():
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Check mixed spectra")')
            pg.click('.fln:has-text("Check mixed spectra")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Check mixed spectra'")
            mgf = b"BEGIN IONS\nTITLE=s1\nPEPMASS=195.09 1000\nCHARGE=1+\nRTINSECONDS=12.5\n138.07 40\n110.07 100\nEND IONS\nBEGIN IONS\nTITLE=s2\nPEPMASS=300.2\n100 5\nEND IONS\n"
            pg.set_input_files('.fl-insp input[type=file][accept]', files=[{"name": "ms2.mgf", "mimeType": "text/plain", "buffer": mgf}])
            attendi(pg, "document.querySelectorAll('.flx-sticks .flx-stick').length === 2")
            assert pg.locator(".flx-sticks").get_attribute("role") == "img" and "2 peaks" in pg.locator(".flx-sticks").get_attribute("aria-label")
            assert pg.locator(".flx-one .flx-table td").nth(2).inner_text() == "110.07"
            pg.select_option('.flx-res select[aria-label="Spectrum to draw"]', "1"); attendi(pg, "document.querySelectorAll('.flx-sticks .flx-stick').length === 1")
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("results: an MGF file lists its spectra, draws one as sticks and gives its peak table", mgf_view)
        def tabui_views():
            # the products table with its conditions (bars + data table), and the reactivity of the parent as a table of atoms
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Transformation products")'); pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Where it reacts")')
            pg.click('.fln:has-text("Link product spectra")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Link product spectra'")
            tp = b'{"schema":"tptab/1","times":[0,30],"rows":[{"row":1,"id":7,"mz":200.1,"rt":4.5,"status":0,"kinship":3,"via":1,"n_det":4,"max_area":1500,"tmax":30,"in_t0":false,"in_blank":false},{"row":2,"id":8,"mz":216.1,"rt":5.1,"status":0,"kinship":2,"via":1,"n_det":3,"max_area":800,"tmax":30,"in_t0":false,"in_blank":false}],"gruppi":{"format":"gruppi/1","conditions":[{"id":"O2","parent":{"k":0.05,"half_life":13.9,"remaining":0.2}},{"id":"N2","parent":{}}],"rows":[{"row":1,"tp_name":"TP 7","mz":200.1,"rt":4.5,"neutral_formula":"C8H9","cond":[{"present":true,"area":900,"ratio_ref":1},{"present":false,"area":0}]},{"row":2,"tp_name":"TP 8","mz":216.1,"rt":5.1,"neutral_formula":"C8H9O","cond":[{"present":true,"area":300,"ratio_ref":1},{"present":true,"area":120}]}],"sets":[{"conditions":["O2"],"rows":[1]},{"conditions":["O2","N2"],"rows":[2]}]}}'
            pg.set_input_files('.fl-insp input[type=file]', files=[{"name": "tp.json", "mimeType": "application/json", "buffer": tp}])
            attendi(pg, "document.querySelectorAll('.flx-sticks .flx-bar').length === 2")
            assert "Probable" in pg.inner_text(".flx-res-body") and "present, area 900" in pg.inner_text(".flx-res-body") and "absent" in pg.inner_text(".flx-res-body")
            assert pg.locator(".flx-sticks[aria-label$='in each condition']").get_attribute("role") == "img" and pg.locator(".flx-one:has(.flx-sticks[aria-label$='in each condition']) .flx-table td").nth(1).inner_text() == "900"
            # one selection: a click on a product (or Enter on the focused row) draws it in the conditions block and marks the sets it belongs to; the picker answers the other way
            rows = ".flx-res-body .flx-table tbody tr[data-id]"
            pg.click(rows + '[data-id="2"]'); attendi(pg, "document.querySelector('.flx-res-body select').value === '1'")
            assert "TP 8" in pg.locator(".flx-sticks[aria-label$='in each condition']").get_attribute("aria-label") and pg.locator(".flx-res-body tr.flx-sel").count() >= 2
            sets = ".flx-res-body tr[data-members]"
            assert pg.locator(sets + ".flx-member").count() == 1 and "O2 + N2" in pg.locator(sets + ".flx-member").inner_text()
            pg.locator(rows + '[data-id="1"]').first.focus(); pg.keyboard.press("Enter")
            attendi(pg, "document.querySelector('.flx-res-body select').value === '0'")
            assert "O2" == pg.locator(sets + ".flx-member td").first.inner_text()
            pg.select_option('.flx-res-body select[aria-label="Product to draw"]', "1")
            attendi(pg, "document.querySelector('.flx-res-body tr[data-id=\"2\"].flx-sel') !== null")
            pg.click('.flx-res button[aria-label="Close the results"]')
            pg.click('.fln:has-text("Where it reacts")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Where it reacts'")
            re_ = b'{"format":"reatt/1","atoms":[{"i":0,"el":6,"heavy_index":0,"q":-0.1,"f_plus":0.1,"f_zero":0.2,"f_minus":0.3333,"dual":0}]}'
            pg.set_input_files('.fl-insp input[type=file]', files=[{"name": "reatt.json", "mimeType": "application/json", "buffer": re_}])
            attendi(pg, "document.querySelectorAll('.flx-res-body .flx-table tbody tr').length === 1")
            assert pg.locator(".flx-res-body .flx-table td").nth(6).inner_text() == "0.333"
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("results: a products file shows its table, the conditions with a chart and its data, and a reactivity file shows its atoms", tabui_views)
        FAKE = "".join(l for l in (HERE / "flow_desktop_fake.js").open() if not l.startswith("//"))      # the app is faked at the native routes flow/live/*
        def desktop_run():
            # the page asks the native routes flow/live/*, says on each step what its events say, and lists what each step did from the prov/1 `finish` answers
            pg.evaluate(FAKE); pg.evaluate("window.__missing = '*'")
            pg.click('#flx-run'); slots = ".fl-slots li"
            go = 'button[aria-label="Run the plan on this computer"]'
            attendi(pg, "document.querySelector('.fl-cli .fl-note').textContent.includes('Data tab')")      # Run started the first run by itself
            pg.evaluate("window.__missing = ''"); attendi(pg, "/^0 steps reused, \\d+ to compute$/.test(document.querySelector('.fl-cli [data-reuse]').textContent)")
            assert "flow/reuse" in pg.evaluate("window.__ops"); n = pg.evaluate("document.querySelector('.flx-list').__flow.plan.steps.length"); pg.evaluate("window.__reused = %d" % n)
            pg.click(go); attendi(pg, "document.querySelector('.fl-slots li').textContent.includes('done in 0.9 s')")      # while the run goes: the first step is done, the others wait
            assert "Running" in pg.inner_text(".fl-cli .fl-note") and "waiting" in pg.inner_text(slots + ":last-child")
            attendi(pg, "document.querySelector('.fl-cli .fl-note').textContent.startsWith('Finished')")
            assert "done in 1.2 s" in pg.inner_text(slots + ":first-child") and "reused" in pg.inner_text(slots + ":last-child") and "1 steps reused" in pg.inner_text(".fl-cli .fl-note") and "/tmp/mzlab-flow/out" in pg.inner_text(".fl-cli .fl-note")
            attendi(pg, "document.querySelector('.fl-cli [data-reuse]').textContent === '%d steps reused, 0 to compute'" % n); pg.click('button[aria-label="Fit all steps in view"]'); pg.click('.fln:has-text("Build feature table")'); pg.click('.fl-insp button[aria-label="Open t.csv"]')
            attendi(pg, "document.querySelector('.flx-table td') && document.querySelector('.flx-table td').textContent === '1'")
        step("desktop: «Run on this computer» says which file to open first, then lists each step as done", desktop_run)
        def desktop_cards():
            attendi(pg, "[...document.querySelectorAll('.fln-lw')].some(n => n.textContent === 'done in 1.2 s')")
            assert pg.evaluate("[...document.querySelectorAll('.fln-live')].every(n => n.hidden || n.querySelector('.fln-lm svg'))")
            assert "flow/live/events" in pg.evaluate("window.__ops") and "flow/progress" not in pg.evaluate("window.__ops")
            pg.click("#flx-live-pause"); attendi(pg, "window.__ops.includes('flow/live/pause')")
            attendi(pg, "document.querySelector('#flx-live-pause').getAttribute('aria-pressed') === 'true' && document.querySelector('#flx-live-pause').textContent.includes('Resume')")     # the bar follows the answer of the engine
            pg.click("#flx-live-pause"); attendi(pg, "window.__ops.includes('flow/live/resume')")
            pg.evaluate("() => { const S = document.querySelector('.flx-list').__flow, i = S.plan.steps[S.plan.steps.length - 1]; i.params = i.params || {}; i.params.min_height = 7; S.refresh(); }"); attendi(pg, "window.__ops.includes('flow/live/update')")
            pg.evaluate("() => { const S = document.querySelector('.flx-list').__flow, i = S.plan.steps[S.plan.steps.length - 1]; delete i.params.min_height; S.refresh(); }")
        step("desktop: the cards say what each step does, a change goes to the engine as an update, Pause and Resume reach it", desktop_cards)
        def desktop_data_files():
            # the files open in Data become the scan inputs; the app holds a.mzML only, so b.mzML is left out and said so, the run goes on
            pg.evaluate("async () => { await FLOW.desktopLive.drop(); }")
            pg.evaluate(FAKE); pg.evaluate("() => { if (typeof E === 'undefined') window.E = { files: [] }; E.files = [{ file: 'a.mzML' }, { file: 'b.mzML' }]; window.__missing = 'b.mzML'; window.__quick = true; }")
            pg.click('button[aria-label="Run the plan on this computer"]'); attendi(pg, "document.querySelector('.fl-cli .fl-note').textContent.startsWith('Finished')")
            note = pg.inner_text(".fl-cli .fl-note"); sent = pg.evaluate("window.__sent")
            assert "1 file excluded" in note and "b.mzML" in note, note
            assert len(sent) == 2 and "a.mzML" in sent[0] and "b.mzML" in sent[0] and "b.mzML" not in sent[1] and "a.mzML" in sent[1], sent
            w = pg.evaluate("""() => { if (typeof ST === 'undefined') window.ST = { files: [] }; ST.files = [{ name: 'a.mzML', spectrum_mode: 'profile' }]; const none = FLOW.runPlanFromData(FLOW.current).warns, d = FLOW.current.plan.steps.find(x => x.step.startsWith('features.detect@')); d.algorithm = 'centwave'; const r = FLOW.runPlanFromData(FLOW.current); delete d.algorithm; ST.files = []; return none.length ? ['unexpected'] : r.warns; }""")
            assert w and all("centroid" in x for x in w) and "a.mzML" in w[0], w
        step("desktop: the files open in Data become the plan inputs, a file the app lacks is left out and listed", desktop_data_files)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
