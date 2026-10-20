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
        step("canvas: Advanced shows every step and the palette; Enter selects, arrows move the view, reduced motion stops the animation", advanced_and_keys)
        def sheets():
            pg.click('[data-tab="plan"]'); assert '"format": "mzflow/2"' in pg.inner_text(".flx-plan")
            pg.click('[data-tab="list"]'); attendi(pg, "document.querySelectorAll('.fl-card').length >= 3"); pg.click('[data-tab="canvas"]')
        step("canvas: the plan view and the list view show the same plan", sheets)
        def results_from_disk():
            pg.click('.fln:has-text("Build feature table")'); attendi(pg, "document.querySelector('.fl-insp h3').textContent === 'Build feature table'")
            pg.set_input_files('.fl-insp input[type=file]', files=[{"name": "peaks.csv", "mimeType": "text/csv", "buffer": b'mz,rt,area\n241.3,3.0,"1,5"\n257.3,4.2,7\n'}])
            attendi(pg, "document.querySelectorAll('.flx-res-body .flx-tablewrap')[1].querySelectorAll('tbody tr').length === 2")
            data = '.flx-res-body > .flx-tablewrap:last-child .flx-table'
            assert pg.locator(data + " th").all_inner_texts() == ["mz", "rt", "area"] and pg.locator(data + " td").nth(2).inner_text() == "1,5"
            # the column summary: one row per column, numbers counted from the whole file
            pg.click('.flx-qc summary'); qc = '.flx-qc .flx-table tbody tr'
            got = pg.locator(qc).nth(0).inner_text().split(); assert pg.locator(qc).count() == 3 and got == ["mz", "2", "0", "241.3", "257.3"], got
            assert pg.locator(".flx-res").get_attribute("aria-label") == "Results" and pg.evaluate("document.activeElement.closest('.flx-res') !== null")
            pg.click('.flx-res button[aria-label="Close the results"]'); assert pg.locator(".flx-res").count() == 0
        step("results: a table chosen from disk opens under the canvas as a real table, readable by keyboard and closable", results_from_disk)
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
            pg.click('.fln:has-text("Build feature table")'); pg.click('.fl-insp button[aria-label="Open t.csv"]')
            attendi(pg, "document.querySelector('.flx-table td') && document.querySelector('.flx-table td').textContent === '1'")
        step("desktop: «Run on this computer» says which file to open first, then lists each step as done", desktop_run)
        b.close()
finally:
    srv.terminate()
for s in steps: print(s)
print("page errors:", errs)
sys.exit(1 if any(s[1] != "ok" for s in steps) or errs else 0)
