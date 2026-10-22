"""Flow «This browser» with the MSn steps (flow/lazy/flow-web-msn.js): the template «Fragmentation tree of a standard» runs on the infusion open in the Data tab with the engine of the page
(api/msn_grafo and api/msn_grafo/export, the .wasm msngrafo): the graph is the text of `mzlab-cli msn-graph`, the library is MSP with the formula and the loss of each explained peak, the
table is the CSV of the modifications, each step reports its events and keeps its text in OPFS; without the infusion open the run says so, a refusal of the memory says to use the desktop app,
a plan with a molecule name and no ion asks for the ion. The file is the synthetic one of TP_Mine/tests/msn_synth.py. Needs the site (python3 tools/prepara_sito.py) with the .wasm msngrafo.
Run from the repo root: python3 tests_e2e/e2e_flow_msn.py"""
import json, re, shutil, subprocess, sys
sys.path.insert(0, __import__("os").path.dirname(__import__("os").path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
sys.path.insert(0, str(ROOT))
from TP_Mine.tests.msn_synth import write_msn, caffeine_nodes

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))

D = synth("flowmsn")
F = write_msn(D / "MSn_synth_t01.mzML", caffeine_nodes(), n_scans=12)
ION = "C8H11N4O2+"
cli = subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(F), "--formula", ION], cwd=ROOT, check=True, capture_output=True, encoding="utf-8").stdout
mask = lambda t: re.sub(r'"sha256_idx":"[0-9a-f]*"', '"sha256_idx":""', t)
RUN = """async (patch) => {
  const S = document.querySelector('.flx-list').__flow, ev = [], on = e => ev.push(e.detail);
  for (const inst of S.plan.steps) if (inst.step.startsWith('spectra.msn_graph')) inst.params = Object.assign({}, inst.params, patch);
  document.addEventListener('flow:web-step', on);
  let err = null, res = null;
  try { res = await FLOW.runners.web.run(S); } catch (e) { err = { key: e.key, params: e.params }; }
  document.removeEventListener('flow:web-step', on);
  return { ev, err, files: res && res.files };
}"""
r = Run(port=8979, wd="/tmp/wd_flowmsn")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("localStorage.setItem('mzlab.admin','1')"); pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
        load(pg, [F], 6000, 1)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web", timeout=120000)
        pg.click('button[aria-label="Start from a template"]'); pg.click('.flx-tc[data-tpl="msn"]')
        attendi(pg, "document.querySelector('.flx-list') && document.querySelector('.flx-list').__flow && document.querySelector('.flx-list').__flow.plan.steps.length === 3")
        got = {}
        def runs():
            got["r"] = x = pg.evaluate(RUN, {"formula": ION})
            assert x["err"] is None, x["err"]
            by = {e["passo"]: [] for e in x["ev"]}
            for e in x["ev"]: by[e["passo"]].append(e["stato"])
            assert len(by) == 3 and all(s[0] == "waiting" and s[-1] == "done" and "running" in s for s in by.values()), by
            assert all(e["via"] == "browser" for e in x["ev"]), x["ev"]
            assert [f["name"].rsplit(".", 1)[1] for f in x["files"]] == ["json", "msp", "csv"], [f["name"] for f in x["files"]]
        step("the template runs in the browser on the open infusion: three steps, three texts", runs)
        def graph():
            t = got["r"]["files"][0]["text"]
            assert mask(t) == mask(cli), (len(t), len(cli))
        step("the graph is the text of mzlab-cli msn-graph", graph)
        def library():
            t = got["r"]["files"][1]["text"]
            assert "Name:" in t and "Num Peaks:" in t and "C8H11N4O2" in t, t[:300]
        step("the library is MSP with the formulas of the explained peaks", library)
        def table():
            t = got["r"]["files"][2]["text"]
            assert t.startswith("from,from_mz,from_formula,to,to_mz,to_formula,loss_mass,loss_formula,loss_ppm"), t[:200]
            assert len(t.splitlines()) > 1, t
        step("the table is the CSV of the modifications", table)
        def kept():
            names = pg.evaluate("""(async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'); const o = []; for await (const [n] of d.entries()) o.push(n); return o })()""")
            assert len([n for n in names if n.endswith((".json", ".msp", ".csv"))]) >= 3, names
        step("each text is kept in OPFS (mzlab/flow/)", kept)
        def no_ion():
            x = pg.evaluate(RUN, {"formula": "", "molecule": "Caffeine"})
            assert x["err"] and x["err"]["key"] == "err.flow.msn.molecule", x["err"]
        step("a molecule name without the ion asks for the ion", no_ion)
        def memory():
            pg.evaluate("window.__f = window.fetch; window.fetch = (u, i) => String(u).includes('api/msn_grafo') ? Promise.resolve(new Response(JSON.stringify({ error: 'x', error_key: 'error.memoryBudget', params: {} }))) : window.__f(u, i)")
            x = pg.evaluate(RUN, {"formula": ION})
            pg.evaluate("window.fetch = window.__f")
            assert x["err"] and x["err"]["key"] == "err.flow.msn.desktop", x["err"]
            assert "desktop app" in pg.evaluate("FLOW.message({ key: 'err.flow.msn.desktop', params: {} })"), "message"
        step("a refusal of the memory says to use the desktop app", memory)
        def closed():
            pg.evaluate("window.__files = E.files; E.files = []")
            x = pg.evaluate(RUN, {"formula": ION})
            pg.evaluate("E.files = window.__files")
            assert x["err"] and x["err"]["key"] == "err.flow.msn.open", x["err"]
            assert "Data tab" in pg.evaluate("FLOW.message({ key: 'err.flow.msn.open', params: { input: 'x.mzML' } })")
        step("without the infusion open the run asks to open it in Data", closed)
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
