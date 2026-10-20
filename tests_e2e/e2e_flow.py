"""Flow, the graphical workflow page (docs/agenti/flow-grafica.md): it exists only in the administrator mode (localStorage «mzlab.admin» = «1»), the palette comes from the three lists of
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
        def hidden():
            attendi(pg, "document.readyState === 'complete' && !!document.querySelector('#nav')")
            assert pg.locator(TAB).count() == 0 and pg.locator("#v-flow").count() == 0
            assert not [r for r in reqs if "flow-" in r or "steps-attributio" in r], "Flow must not be downloaded for a normal user"
        step("a normal user has no Flow tab and downloads nothing of it", hidden)
        pg.context.close()

        pg = fresh(True, reduced=True)
        pg.wait_for_selector(TAB, timeout=60000); pg.evaluate(COUNT)
        pg.click(TAB); pg.wait_for_selector("#flx-root", timeout=30000)
        # the list view is the editor under the canvas: the same plan, started empty here; the setup clicks are not part of the budget below
        pg.click('[data-tab="list"]'); pg.click('button[aria-label="Start a new plan"]'); pg.evaluate("document.querySelector('.flx-pal').remove()")
        def palette():
            assert pg.locator(".fl-it").count() == 26, pg.locator(".fl-it").count()
            assert pg.locator(".fl-pal details").count() == 3
            pg.fill("#fl-filter", "formula"); attendi(pg, "[...document.querySelectorAll('.fl-it')].filter(b => !b.hidden).length >= 2")
            assert pg.locator(".fl-it:visible").count() < 26
            pg.fill("#fl-filter", "")
        step("palette: 26 steps in three packages, the filter narrows it", palette)
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
        def parameters():
            pg.fill("#fl-p-detect-floor", "5000"); pg.keyboard.press("Tab")
            attendi(pg, "!!document.querySelector('#fl-card-detect button[aria-label^=\"Back to the default\"]')")
            with pg.expect_download() as d: pg.click('button[aria-label="Export the plan as mzflow/2 JSON"]')
            plan = json.loads(Path(d.value.path()).read_text())
            assert plan["steps"][2]["params"] == {"floor": 5000}, plan["steps"][2]
            pg.click('#fl-card-detect button[aria-label^="Back to the default"]')
            attendi(pg, "!document.querySelector('#fl-card-detect button[aria-label^=\"Back to the default\"]')")
            pg.select_option("#fl-alg-detect", "centwave"); attendi(pg, "document.querySelector('#fl-card-detect').textContent.includes('is planned, not available yet')")
            pg.select_option("#fl-alg-detect", "masstrace")
        step("the form: only differences from the default are written; a planned algorithm is refused", parameters)
        def goal():
            attendi(pg, "document.querySelector('.fl-goal').textContent.includes('cannot reach')")
            pg.select_option("#fl-add-input", "spectra"); pg.select_option("#fl-add-input", "library")
            attendi(pg, "document.querySelector('.fl-goal').textContent.includes('does not reach the identification level yet')")
        step("the goal: identification needs spectra and a library, and the page says so", goal)
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
            assert pg.locator('button[aria-label="Compare algorithms of detect"]').count() == 0     # one available algorithm only
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
            pg.click('button[aria-label="Export and run with the command line"]')
            assert "mzlab-cli --example flow" in pg.inner_text(".fl-cli")
        step("«Run with the CLI» shows the command of the native runner", cli)

        pg = fresh(True, reduced=True)
        pg.wait_for_selector(TAB, timeout=60000); pg.evaluate(COUNT)
        pg.click(TAB); pg.wait_for_selector(".fln", timeout=30000)
        def first_screen():
            attendi(pg, "document.querySelectorAll('.fln').length === 2")
            assert "Import data" in pg.inner_text(".fln") and pg.locator(".fl-lines path").count() == 1
            assert pg.locator(".fln .fln-chip.ok").count() == 2 and pg.locator(".flx-tpl.show").count() == 1 and pg.locator(".flx-tc").count() == 4
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
            pg.click('.flx-tc[data-tpl="identify"]')
            attendi(pg, "document.querySelectorAll('.fln').length === 7")
            assert "Ready to run" in pg.inner_text("#flx-status"), pg.inner_text("#flx-status")
            n = pg.evaluate("window.__n"); total = n["click"] + n["key"]
            print(f"MISURA clic_canvas: tab 1 + template 1 = {total} clicks to a complete valid chain; with the {ADMIN} clicks of the logo {total + ADMIN}")
            assert total <= 3, n
        step("canvas: one template click gives a complete valid chain", template)
        def inspector():
            pg.click('.fln:has-text("Match to library")')
            attendi(pg, "document.querySelector('.fl-insp h3') && document.querySelector('.fl-insp h3').textContent === 'Match to library'")
            assert "What this step does" in pg.inner_text(".fl-insp") and pg.locator(".fl-insp button:has-text('Remove step')").count() == 1
            pg.click('.fln:has-text("Import data")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Import data'")
            assert pg.locator(".fl-insp button:has-text('Remove')").count() == 0 and "cannot remove" not in pg.inner_text(".fl-insp") and "part of the base" in pg.inner_text(".fl-insp")
        step("canvas: the inspector explains the step; the base cannot be removed", inspector)
        def needs_and_fix():
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Regulatory lists")')
            attendi(pg, "document.querySelectorAll('.fln').length === 8")
            pg.click('.fln:has-text("Regulatory lists")')
            assert pg.locator(".fln.sel .fln-chip.ok").count() == 1
            pg.click('.fl-insp button:has-text("Remove step")'); attendi(pg, "document.querySelectorAll('.fln').length === 7")
            assert "Removed" in pg.inner_text("#flx-live")
            pg.click('#flx-live button:has-text("Undo")'); attendi(pg, "document.querySelectorAll('.fln').length === 8")
            pg.click('button[aria-label="Start from a template"]'); pg.click('.flx-tc[data-tpl="table"]'); attendi(pg, "document.querySelectorAll('.fln').length === 2")
            pg.click("#flx-add"); pg.click('.flx-pop button:has-text("Confidence level")'); attendi(pg, "document.querySelectorAll('.fln').length === 3")
            pg.click('.fln:has-text("Confidence level")')
            assert "Needs attention" in pg.inner_text(".fln.sel") and pg.locator(".fl-needs li").count() == 2, pg.inner_text(".fl-insp")
            pg.click('.fl-fix:has-text("Molecular formulas")'); attendi(pg, "document.querySelectorAll('.fl-needs li').length === 1")
            assert "Remove" in pg.inner_text(".fl-insp")
        step("canvas: a missing input is said on the card with a one-click fix; removing can be undone", needs_and_fix)
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
            assert not pg.locator('[data-tab="plan"]').is_visible()
            pg.click('[data-mode="advanced"]'); pg.click('[data-tab="plan"]'); assert '"format": "mzflow/2"' in pg.inner_text(".flx-plan")
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
              ATTRIBUTIO_CORE.pool = async () => ({ attributioExport: async (f, r, k) => ({ files: [{ name: 'out_' + k + '.csv', text: 'a;b' }], warnings: [] }), attributioReview: async (f, r, pol, id) => rev(id, id === 1 ? "2" : "3"), attributioDecision: async (f, r, d) => { window.__saved.push(JSON.parse(d)); return { summary: { path: "decisions/feature_decisions.csv" }, text: "csv" }; } });
              const data = { columns: [], rows: [["POS", 1, 200.1, 3.2, 2, "b", "b", "x", "", ""], ["POS", 2, 201.1, 3.4, 3, "a", "b", "y", "", ""]] };
              FLOW.openResult({ type: "annot", label: "demo (campaign folder)", campaign: { root: "demo", files: [], data, summary: {} }, text: async () => "" });
            }""")
            # the engine exports of Attributio hang on the same campaign: six items, one file downloads
            assert pg.locator('.flx-res #at-export:not([disabled])').count() == 1 and pg.locator('.flx-res #at-export-menu button').count() == 6
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
            tp = b'{"schema":"tptab/1","times":[0,30],"rows":[{"row":1,"id":7,"mz":200.1,"rt":4.5,"status":0,"kinship":3,"via":1,"n_det":4,"max_area":1500,"tmax":30,"in_t0":false,"in_blank":false}],"gruppi":{"format":"gruppi/1","conditions":[{"id":"O2","parent":{"k":0.05,"half_life":13.9,"remaining":0.2}},{"id":"N2","parent":{}}],"rows":[{"row":1,"tp_name":"TP 7","mz":200.1,"rt":4.5,"neutral_formula":"C8H9","cond":[{"present":true,"area":900,"ratio_ref":1},{"present":false,"area":0}]}],"sets":[{"conditions":["O2"],"rows":[1]}]}}'
            pg.set_input_files('.fl-insp input[type=file]', files=[{"name": "tp.json", "mimeType": "application/json", "buffer": tp}])
            attendi(pg, "document.querySelectorAll('.flx-sticks .flx-bar').length === 2")
            assert "Probable" in pg.inner_text(".flx-res-body") and "present, area 900" in pg.inner_text(".flx-res-body") and "absent" in pg.inner_text(".flx-res-body")
            assert pg.locator(".flx-sticks").get_attribute("role") == "img" and pg.locator(".flx-one .flx-table td").nth(1).inner_text() == "900"
            pg.click('.flx-res button[aria-label="Close the results"]')
            pg.click('.fln:has-text("Where it reacts")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Where it reacts'")
            re_ = b'{"format":"reatt/1","atoms":[{"i":0,"el":6,"heavy_index":0,"q":-0.1,"f_plus":0.1,"f_zero":0.2,"f_minus":0.3333,"dual":0}]}'
            pg.set_input_files('.fl-insp input[type=file]', files=[{"name": "reatt.json", "mimeType": "application/json", "buffer": re_}])
            attendi(pg, "document.querySelectorAll('.flx-res-body .flx-table tbody tr').length === 1")
            assert pg.locator(".flx-res-body .flx-table td").nth(6).inner_text() == "0.333"
            pg.click('.flx-res button[aria-label="Close the results"]')
        step("results: a products file shows its table, the conditions with a chart and its data, and a reactivity file shows its atoms", tabui_views)
        def desktop_run():
            # the app is faked: the page asks the native route flow/run and lists what each step did from the prov/1 it answers
            pg.evaluate("""() => { window.MZLAB_DESKTOP = true; window.__runs = 0; const real = window.fetch.bind(window);
              window.fetch = async (u, o) => { if (!String(u).includes('/rpc')) return real(u, o);
                if (JSON.parse(o.body).op === 'flow/read') return new Response('a,b\\n1,2\\n', { status: 200 });
                const plan = JSON.parse(JSON.parse(o.body).plan || '{"steps":[]}'); window.__runs++;
                if (window.__runs === 1) return new Response(JSON.stringify({ error_key: 'err.flow.input.missing', params: { input: 'a.mzML' } }), { status: 400 });
                const activity = {}; plan.steps.forEach(x => { activity[x.id] = { step: x.step, ms: 1200, used: [], generated: ['k'] }; });
                return new Response(JSON.stringify({ prov: JSON.stringify({ schema: 'prov/1', entity: { k: { type: 'table', label: 't.csv' } }, activity }), dir: '/tmp/mzlab-flow/out' }), { status: 200 }); }; }""")
            pg.click('button[aria-label="Export the plan and run it"]')
            slots = ".fl-slots li"; assert pg.locator(slots).count() >= 3 and "waiting" in pg.inner_text(slots + ":first-child")
            go = 'button[aria-label="Run the plan on this computer"]'
            pg.click(go); attendi(pg, "document.querySelector('.fl-cli .fl-note').textContent.includes('Data tab')")
            pg.click(go); attendi(pg, "document.querySelector('.fl-cli .fl-note').textContent.startsWith('Finished')")
            assert "done in 1.2 s" in pg.inner_text(slots + ":first-child") and "/tmp/mzlab-flow/out" in pg.inner_text(".fl-cli .fl-note")
            pg.click('button[aria-label="Fit all steps in view"]'); pg.click('.fln:has-text("Build feature table")'); pg.click('.fl-insp button[aria-label="Open t.csv"]')
            attendi(pg, "document.querySelector('.flx-table td') && document.querySelector('.flx-table td').textContent === '1'")
        step("desktop: «Run on this computer» says which file to open first, then lists each step as done", desktop_run)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
