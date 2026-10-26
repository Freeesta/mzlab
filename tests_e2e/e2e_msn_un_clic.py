"""Run on the Fragmentation tree template of Flow (the Run button opens the box, one click in it runs): the infusion is open in Data, the molecule is typed in the box of the first step (ion, adduct and SMILES arrive by themselves) and a single click
on «Run the plan in this browser» gives the graph with the interpretation, the library as MSP, the table of the modifications and, with the library of the user in OPFS, the library search (msncerca/1);
the texts are saved as links. File of TP_Mine/tests/msn_synth.py (no real data). Needs the site with the .wasm msngrafo. Run from the repo root: python3 tests_e2e/e2e_msn_un_clic.py"""
import json, sys
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

D = synth("msnunclic")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
PUT = """async (t) => { const d = await navigator.storage.getDirectory(), w = await (await d.getFileHandle('msnlib.json', { create: true })).createWritable(); await w.write(t); await w.close() }"""
FLOWS = "document.querySelector('.flx-list').__flow"
r = Run(port=8981, wd="/tmp/wd_msnunclic")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("localStorage.setItem('mzlab.admin','1')"); pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
        attendi(pg, "document.getElementById('pick').onchange !== null", 120000)   # the start screen takes a file only once its shell has bound the input
        load(pg, [F], 6000, 1)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web", timeout=120000)
        (None if pg.locator('.flx-tpl.show').count() else pg.click('button[aria-label="Start from a template"]')); pg.click('.flx-tc[data-tpl="msn"]')   # the button toggles the cards: open on the first screen
        attendi(pg, f"document.querySelector('.flx-list') && {FLOWS} && {FLOWS}.plan.steps.length === 3")
        def prepare():
            pg.evaluate(f"""() => {{ const S = {FLOWS}, inst = S.plan.steps[0];
              const box = FLOW.form(inst, FLOW.stepOf(S.reg, inst), () => {{}}); box.id = 'tmpform'; document.body.append(box); }}""")
            pg.wait_for_selector("#tmpform .molc input", timeout=30000)
            pg.fill("#tmpform .molc input", "caffeine"); pg.wait_for_selector("#tmpform .molc li", timeout=30000); pg.click("#tmpform .molc li >> nth=0")
            pg.wait_for_function(f"(() => {{ const p = {FLOWS}.plan.steps[0].params || {{}}; return !!p.formula; }})()", timeout=30000)
            pg.evaluate("document.querySelector('#tmpform').remove()")
        step("the molecule typed in the box fills the ion, the adduct and the SMILES", prepare)
        def one_click():
            pg.click("#flx-run")
            try: pg.wait_for_function("(document.querySelector('.fl-note[role=status]')||{}).textContent && document.querySelector('.fl-note[role=status]').textContent.startsWith('Finished')", timeout=120000)
            except Exception: raise AssertionError("note: " + pg.inner_text(".fl-note[role=status]") + " | " + pg.inner_text(".fl-slots"))
            links = pg.evaluate("[...document.querySelectorAll('.fl-slots a[download]')].map(a => a.download)")
            assert [n.rsplit(".", 1)[1] for n in links] == ["json", "msp", "csv"], links
        step("one click runs graph, library and table and saves three files", one_click)
        def texts():
            got = pg.evaluate("""Promise.all([...document.querySelectorAll('.fl-slots a[download]')].map(a => fetch(a.href).then(r => r.text())))""")
            g, msp, csv = got
            j = json.loads(g)
            assert j["formato"] == "msngrafo/1" and j.get("interpretazione"), list(j)[:12]
            assert "Name:" in msp and "Num Peaks:" in msp
            assert csv.startswith("from,from_mz,from_formula,to,to_mz,to_formula,loss_mass,loss_formula,loss_ppm") and len(csv.splitlines()) > 1
        step("the three texts are the graph with the interpretation, MSP and the CSV of the modifications", texts)
        def search():
            ion = pg.evaluate(f"{FLOWS}.plan.steps[0].params.formula")
            added = pg.evaluate("""(async f => { const r = await fetch('api/msnlib/aggiungi', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ libreria: '', molecola_id: 'STD', k: 0, formula: f }) }); const j = await r.json(); return j.error ? j : j.libreria })""", ion)
            assert isinstance(added, str), added
            pg.evaluate(PUT, added)
            # the scheduler runs only a plan that checks: a query file of the plan and the library of the tree
            pg.evaluate(f"""(() => {{ const S = {FLOWS}, l = S.plan.steps.find(s => s.step.startsWith('msn.library_export')); S.plan.inputs.push({{ name: 'q.mgf', type: 'spectra', sha256: '' }}); S.plan.steps.push({{ id: 'srch', step: 'spectra.match@1', algorithm: 'tree', in: {{ query: 'file:q.mgf', library: l.id + '.library' }}, params: {{}} }}) }})()""")
            pg.click('button[aria-label="Run the plan in this browser"]')
            try: pg.wait_for_function("document.querySelectorAll('.fl-slots a[download]').length === 4", timeout=60000)
            except Exception: raise AssertionError("note: " + pg.inner_text(".fl-note[role=status]") + " | " + pg.inner_text(".fl-slots"))
            hit = pg.evaluate("fetch(document.querySelectorAll('.fl-slots a[download]')[3].href).then(r => r.text())")
            h = json.loads(hit)
            assert h["formato"] == "msncerca/1" and h["candidati"], hit[:200]
        step("with the library of the user in OPFS the same click adds the search: msncerca/1", search)
        errs = [e for e in r.errs if e[0] in ("pageerror",)]
        step("no page errors", lambda: (_ for _ in ()).throw(AssertionError(errs[:3])) if errs else None)
        miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).slice()")
        step("no request left unanswered by the engine", lambda: (_ for _ in ()).throw(AssertionError(miss)) if miss else None)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
