"""Flow, the input node (docs/agenti/flow-web.md): «Add files» and «Add a folder» of the node open the files they read at once, without a visit to the Data tab (they used to wait unopened in the
loading table, so the node stayed «Add a file» and spectra.msn_graph said the infusion was not open); the status line never writes «null»; the message of an infusion that is not open says what to do.
The file is the synthetic MSn one of TP_Mine/tests/msn_synth.py (an mzML: a Thermo .raw goes through the same loaders and the same opening). Needs the site (python3 tools/prepara_sito.py).
Run from the repo root: python3 tests_e2e/e2e_flow_aggiungi_file.py"""
import sys
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
from lib_hr import synth
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

D = synth("flowaggiungi")
(D / "dir").mkdir(exist_ok=True)
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
G = write_msn(D / "dir" / "MSn_synth_t02.mzML", caffeine_nodes(), n_scans=12)
RUN = """async () => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  for (const inst of S.plan.steps) if (inst.step.startsWith('spectra.msn_graph')) inst.params = Object.assign({}, inst.params, { formula: 'C8H11N4O2+' });
  document.addEventListener('flow:web-step', on);
  let err = null;
  try { await FLOW.runners.web.run(S); } catch (e) { err = { key: e.key, msg: String(e && (e.message || e)), text: e.key ? FLOW.message({ key: e.key, params: e.params }) : null }; }
  document.removeEventListener('flow:web-step', on);
  return { ev, err };
}"""

def to_flow(pg):
    pg.evaluate("localStorage.setItem('mzlab.admin','1')"); pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
    pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
    pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web", timeout=120000)
    (None if pg.locator('.flx-tpl.show').count() else pg.click('button[aria-label="Start from a template"]')); pg.click('.flx-tc[data-tpl="msn"]')
    attendi(pg, "document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow && document.querySelector('.flx-list').__flow.plan.steps.length === 3")

def open_input(pg):
    pg.click('.fln[data-key^="file:"]'); attendi(pg, "!!document.querySelector('.fl-insp .fl-data')")

def add(pg, button, target):
    with pg.expect_file_chooser() as fc: pg.click(f".fl-insp button:has-text('{button}')")
    fc.value.set_files(str(target))

r = Run(port=8981, wd="/tmp/wd_flowaggiungi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        to_flow(pg)
        def before():
            open_input(pg)
            assert "No file loaded" in pg.inner_text(".fl-insp"), pg.inner_text(".fl-insp")
            x = pg.evaluate(RUN)
            assert x["err"] and x["err"]["key"] in ("err.flow.msn.open", "err.flow.input.missing"), x["err"]
            assert "open" in x["err"]["text"] and "Add files" in x["err"]["text"] and "undefined" not in x["err"]["text"], x["err"]
        step("without a file the run says the infusion is not open and how to add it", before)
        def files():
            add(pg, "Add files", F)
            attendi(pg, "FLOW.dataFiles().length === 1", 180000)
            assert pg.evaluate("FLOW.dataFiles()[0]") == "MSn_synth_t01.mzML", pg.evaluate("FLOW.dataFiles()")
        step("«Add files» of the input node opens the file (the node no longer waits for a file)", files)
        def node():
            attendi(pg, "!/Add a file/.test(document.querySelector('.fln[data-key^=\"file:\"]').textContent)", 30000)
        step("the node does not say «Add a file» any more", node)
        def run():
            x = pg.evaluate(RUN)
            assert x["err"] is None, x["err"]
            by = {}
            for e in x["ev"]: by.setdefault(e["passo"], []).append(e["stato"])
            assert len(by) == 3 and all(s[-1] == "done" for s in by.values()), by
        step("spectra.msn_graph opens the file and the steps after it are done, none skipped", run)
        def graph_result():
            pg.click("#flx-run"); pg.click('.fl-meta button:has-text("Run in this browser")')
            attendi(pg, "(() => { const S = document.querySelector('.flx-list').__flow; return Object.values(S.results || {}).some(l => l.some(r => /\\.json$/.test(r.label))); })()", 120000)
        step("after the run the graph joins the Results of its step", graph_result)
        def no_null():
            attendi(pg, "(document.getElementById('flx-live') || {}).textContent", 30000)      # the status line spoke about a step
            t = pg.inner_text("#flx-live")
            assert t.strip() and "null" not in t, repr(t)
        step("the status line of the steps has words and no «null»", no_null)
        # a new page: the folder
        r.b.close(); pg = r.page(p)
        to_flow(pg)
        def folder():
            assert pg.evaluate("FLOW.dataFiles().length") == 0
            open_input(pg)
            add(pg, "Add a folder", D / "dir")
            attendi(pg, "FLOW.dataFiles().length === 1", 180000)
            assert pg.evaluate("FLOW.dataFiles()[0]") == "MSn_synth_t02.mzML", pg.evaluate("FLOW.dataFiles()")
        step("«Add a folder» of the input node opens the files of the folder", folder)
        def run2():
            x = pg.evaluate(RUN)
            assert x["err"] is None, x["err"]
        step("the infusion from the folder runs without error", run2)
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
