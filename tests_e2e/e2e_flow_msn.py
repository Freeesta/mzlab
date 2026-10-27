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
_cli = subprocess.run(["cargo", "run", "--release", "--locked", "-q", "-p", "mzlab-cli", "--", "msn-graph", str(F), "--formula", ION, "--interpreta"], cwd=ROOT, capture_output=True, encoding="utf-8")
# a failure of the reference command must show its stderr in the log (a bare CalledProcessError hid the cause)
if _cli.returncode: sys.exit("msn-graph reference failed (exit %d):\n%s" % (_cli.returncode, _cli.stderr[-3000:]))
cli = _cli.stdout
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
REF = """async (patch) => {
  const S = document.querySelector('.flx-list').__flow;
  for (const inst of S.plan.steps) if (inst.step.startsWith('spectra.msn_graph')) inst.params = Object.assign({}, inst.params, patch);
  try { return { err: null, files: (await FLOW.webMsn.run(S)).files }; } catch (e) { return { err: { key: e.key, params: e.params }, files: null }; }
}"""
r = Run(port=8979, wd="/tmp/wd_flowmsn")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("localStorage.setItem('mzlab.admin','1')"); pg.reload(); pg.wait_for_selector("#drop", timeout=120000)
        attendi(pg, "document.getElementById('pick').onchange !== null", 120000)
        load(pg, [F], 6000, 1)
        attendi(pg, "E.panels.some(p=>p.type==='spec' && p._a && p._a.data && p.filt && p.filt.startsWith('msn:'))", 60000)
        pg.wait_for_selector('#nav button[data-v="flow"]', timeout=120000); pg.click('#nav button[data-v="flow"]')
        pg.wait_for_function("window.FLOW && FLOW.runners && FLOW.runners.web", timeout=120000)
        (None if pg.locator('.flx-tpl.show').count() else pg.click('button[aria-label="Start from a template"]')); pg.click('.flx-tc[data-tpl="msn"]')   # the button toggles the cards: open on the first screen
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
        def fractions():
            ev = pg.evaluate("""(async () => JSON.parse(await (await fetch('api/flow/sched/events', { method: 'POST', body: JSON.stringify({ since: 0 }) })).text()))()""")
            for inst in ("msngraph", "librarye", "tableexp"):
                fr = [e["frac"] for e in ev if e.get("kind") == "job" and e.get("instance") == inst and e.get("state") == "running" and e.get("frac") is not None]
                assert any(0 < f < 1 for f in fr), (inst, fr, [(e.get("kind"), e.get("state"), e.get("frac")) for e in ev if e.get("instance") == inst])
                assert fr == sorted(fr), (inst, fr)
        step("each MSn job reports the stages it really went through (a fraction between 0 and 1, never backwards)", fractions)
        def same():
            ref = pg.evaluate(REF, {"formula": ION})
            assert ref["err"] is None, ref["err"]
            a, b = got["r"]["files"], ref["files"]
            assert [f["name"] for f in a] == [f["name"] for f in b], (a and [f["name"] for f in a], [f["name"] for f in b])
            for fa, fb in zip(a, b): assert mask(fa["text"]) == mask(fb["text"]), "the scheduler differs from the functions of the MSn steps on " + fa["name"]
        step("a plan of MSn steps only goes through the scheduler and gives the same texts as the functions of the steps", same)
        def graph():
            t = got["r"]["files"][0]["text"]
            assert mask(t) == mask(cli), (len(t), len(cli))
        step("the graph is the text of mzlab-cli msn-graph --interpreta (the template starts with the interpretation on)", graph)
        def molecule_box():
            pg.evaluate("""() => { const S = document.querySelector('.flx-list').__flow, inst = S.plan.steps[0]; inst.params = { interpreta: true };
              const box = FLOW.form(inst, FLOW.stepOf(S.reg, inst), () => {}); box.id = 'tmpform'; document.body.append(box); }""")
            pg.wait_for_selector("#tmpform .molc input", timeout=30000)
            pg.fill("#tmpform .molc input", "caffeine"); pg.wait_for_selector("#tmpform .molc li", timeout=30000); pg.click("#tmpform .molc li >> nth=0")
            pg.wait_for_function("(() => { const p = document.querySelector('.flx-list').__flow.plan.steps[0].params || {}; return !!p.formula; })()", timeout=30000)
            p = pg.evaluate("document.querySelector('.flx-list').__flow.plan.steps[0].params")
            assert p["formula"].split(" ")[0] and "[M" in p["formula"] and p.get("molecule") and p.get("smiles"), p
            pg.evaluate("document.querySelector('#tmpform').remove()")
        step("the molecule box of the step fills the ion with its adduct, the name and the SMILES", molecule_box)
        def search():
            added = pg.evaluate("""(async () => { const r = await fetch('api/msnlib/aggiungi', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ libreria: '', molecola_id: 'STD', k: 0, formula: '%s' }) }); const j = await r.json(); return j.error ? j : j.libreria })()""" % ION)
            assert isinstance(added, str), added
            put = """async (t) => { const d = await navigator.storage.getDirectory(), w = await (await d.getFileHandle('msnlib.json', { create: true })).createWritable(); await w.write(t); await w.close() }"""
            pg.evaluate(put, "")
            x = pg.evaluate("""async () => { const S = document.querySelector('.flx-list').__flow, ev = [];
              const l = S.plan.steps.find(s => s.step.startsWith('msn.library_export')); S.plan.inputs.push({ name: 'q.mgf', type: 'spectra', sha256: '' }); S.plan.steps.push({ id: 'srch', step: 'spectra.match@1', algorithm: 'tree', in: { query: 'file:q.mgf', library: l.id + '.library' }, params: {} });
              try { await FLOW.runners.web.run(S); return { ok: 1 } } catch (e) { return { key: e.key } } finally { S.plan.steps.pop(); S.plan.inputs.pop() } }""")
            assert x.get("key") == "err.flow.msn.library", x
            pg.evaluate(put, added)
            x = pg.evaluate("""async () => { const S = document.querySelector('.flx-list').__flow;
              const l = S.plan.steps.find(s => s.step.startsWith('msn.library_export')); S.plan.inputs.push({ name: 'q.mgf', type: 'spectra', sha256: '' }); S.plan.steps.push({ id: 'srch', step: 'spectra.match@1', algorithm: 'tree', in: { query: 'file:q.mgf', library: l.id + '.library' }, params: {} });
              try { const r = await FLOW.runners.web.run(S); return { text: r.files.find(f => f.id === 'srch').text } } catch (e) { return { key: e.key } } finally { S.plan.steps.pop(); S.plan.inputs.pop() } }""")
            assert "text" in x, x
            j = json.loads(x["text"])
            assert j["formato"] == "msncerca/1" and j["opzioni"]["min_prove"] == 3 and j["candidati"], x["text"][:300]
        step("the library search of spectra.match (tree) posts the library of the card and gives msncerca/1", search)
        def library():
            t = got["r"]["files"][1]["text"]
            assert "Name:" in t and "Num Peaks:" in t and "C8H11N4O2" in t, t[:300]
        step("the library is MSP with the formulas of the explained peaks", library)
        def table():
            t = got["r"]["files"][2]["text"]
            assert t.startswith("from,from_mz,from_formula,to,to_mz,to_formula,loss_mass,loss_formula,loss_ppm"), t[:200]
            assert len(t.splitlines()) > 1, t
        step("the table is the CSV of the modifications", table)
        def graph_view():
            g = json.loads(got["r"]["files"][0]["text"])
            # Run in the page opens the graph by itself under the canvas, and the three texts join the Results of their nodes
            pg.click("#flx-run")
            pg.wait_for_selector(".flx-res .ig-node", timeout=60000)
            assert pg.locator(".flx-res .ig-node").count() == len(g["nodi"]), "one box per ion"
            assert pg.locator(".flx-res [aria-pressed=true]").first.inner_text() == "Graph"
            attendi(pg, "document.activeElement && document.activeElement.matches('.flx-res h3')", 10000)
            kinds = pg.evaluate("Object.values(document.querySelector('.flx-list').__flow.results).flat().map(r => r.type).sort()")
            assert kinds == ["library", "report", "table"], kinds
            pg.locator(".flx-res .ig-node").nth(1).click()
            pg.wait_for_selector(".flx-res .ig-cands", timeout=10000)   # the panel of the ion is the one of the Fragmentation graph tab of Data
            if not pg.locator(".flx-res .ig-use:not([disabled])").count():   # «Use» is offered only to an ion with several candidates
                for i in range(2, pg.locator(".flx-res .ig-node").count()):
                    pg.locator(".flx-res .ig-node").nth(i).click(); pg.wait_for_selector(".flx-res .ig-cands", timeout=10000)
                    if pg.locator(".flx-res .ig-use:not([disabled])").count(): break
            if pg.locator(".flx-res .ig-use:not([disabled])").count():
                pg.locator(".flx-res .ig-use:not([disabled])").first.click()
                sc = pg.evaluate("(document.querySelector('.flx-list').__flow.plan.steps.find(s => s.step.startsWith('spectra.msn_graph')).params || {}).scelte")
                assert sc and "=" in sc, sc
            pg.click(".flx-res [aria-pressed=false]")
            assert pg.locator(".flx-res table").count() >= 1
            assert pg.evaluate("window.MZLAB_LR_MISS || []") == [], "no route the engine could not serve"
            pg.evaluate("document.querySelector('.flx-res').remove()")
        step("the graph view draws one box per ion and «Use» writes the choice in scelte", graph_view)
        def cards():
            # Raw files → Fragmentation graph → {Export, View graph}; no id on a card; the view card is on once the run made the graph
            names = pg.evaluate("[...document.querySelectorAll('.fln .fln-t')].map(n => n.textContent).sort()")
            assert {"Export", "Fragmentation graph", "Raw files", "View graph"} <= set(names), names
            assert pg.locator(".fl-lines path").count() >= 3, "an arc from Raw files, one to Export, one to View graph"
            shown = pg.inner_text("#flx-root")
            assert not any(i in shown for i in ("spectra.msn_graph", "msn.library_export", "msn.table_export")), "no step id on the page"
            pos = pg.evaluate("Object.fromEntries([...document.querySelectorAll('.fln')].map(n => [n.querySelector('.fln-t').textContent, n.offsetLeft]))")
            assert pos["Raw files"] < pos["Fragmentation graph"] < pos["Export"] == pos["View graph"], pos
            assert pg.locator('.fln[data-key^="view:"]:not([aria-disabled])').count() == 1, "the view is on after the run"
            pg.locator('.fln[data-key^="view:"]').click()
            pg.wait_for_selector(".flx-res .ig-node", timeout=30000)
            assert pg.evaluate("document.activeElement && document.activeElement.closest('.flx-res') !== null"), "the focus is inside the view"
            g = json.loads(got["r"]["files"][0]["text"])
            pg.click('.flx-res button:has-text("Radial")'); pg.wait_for_selector(".flx-res .ig-node", timeout=10000)
            assert pg.locator(".flx-res .ig-node").count() == len(g["nodi"]), "the radial view draws every ion"
            pg.locator(".flx-res .ig-node").first.click(); pg.wait_for_selector(".flx-res .ig-cands", timeout=10000)
            pg.click('.flx-res button:has-text("Open in Data")')
            attendi(pg, "document.getElementById('ig-wrap') && document.querySelector('#ig-wrap .ig-node')", 60000)
            pg.click('#nav button[data-v="flow"]'); pg.evaluate("document.querySelector('.flx-res') && document.querySelector('.flx-res').remove()")
        step("the cards: Raw files, Fragmentation graph, Export and View graph; the view opens with Tree/Radial, the ion panel and Open in Data", cards)
        def empty():
            pg.evaluate("window.__f = window.__f || window.fetch; window.fetch = (u, i) => String(u).includes('api/msn_grafo') ? Promise.resolve(new Response(JSON.stringify({ formato: 'msngrafo/1', nodi: [], archi: [], spettri: [] }))) : window.__f(u, i); null")
            try: x = pg.evaluate(RUN, {"formula": ION})
            finally: pg.evaluate("window.fetch = window.__f; null")
            assert x["err"] and x["err"]["key"] == "err.flow.msn.empty", x["err"]
        step("a file with no MSn spectra gets a clear message", empty)
        def kept():
            names = pg.evaluate("""(async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow'); const o = []; for await (const [n] of d.entries()) o.push(n); return o })()""")
            assert len([n for n in names if n.endswith((".json", ".msp", ".csv"))]) >= 3, names
        step("each text is kept in OPFS (mzlab/flow/)", kept)
        def mixed():
            # the same plan run in order by the functions of the MSn steps, outside the scheduler, is the reference: earlier steps of this test wrote the molecule into the plan
            ref = pg.evaluate(REF, {"formula": ION})
            assert ref["err"] is None, ref["err"]
            # an index step beside the MSn steps: the plan is no longer MSn only, so the steps go through the scheduler (executor `msn` of flow-live-jobs.js)
            pg.evaluate("""() => { const S = document.querySelector('.flx-list').__flow, g = S.plan.steps.find(s => s.step.startsWith('spectra.msn_graph')); S.plan.steps.push({ id: 'ixmix', step: 'index.mzidx@1', in: { source: g.in.source } }); }""")
            try:
                x = pg.evaluate(RUN, {"formula": ION})
                assert x["err"] is None, x["err"]
                last = {}
                for e in x["ev"]: last[e["passo"]] = e
                msn = [e for k, e in last.items() if k != "ixmix"]
                assert len(msn) == 3 and all(e["stato"] in ("done", "reused") and e["via"] == "browser" for e in msn), last
                assert [f["name"].rsplit(".", 1)[1] for f in x["files"]] == ["json", "msp", "csv"], x["files"]
                assert [mask(f["text"]) for f in x["files"]] == [mask(f["text"]) for f in ref["files"]], "the texts of the scheduler differ from the runner of the MSn plan"
                # a job of the scheduler that finishes (Auto-run too) puts its texts in the Results and opens the graph without the run box
                assert pg.evaluate("Object.values(document.querySelector('.flx-list').__flow.results).flat().filter(r => r.type === 'report').length") >= 1, "the graph joined the Results"
                pg.wait_for_selector(".flx-res .ig-node", timeout=30000)
            finally: pg.evaluate("() => { const S = document.querySelector('.flx-list').__flow; S.plan.steps = S.plan.steps.filter(s => s.id !== 'ixmix'); }")
        step("a plan with an index step besides the MSn steps runs them in the scheduler, with the same texts", mixed)
        def no_ion():
            x = pg.evaluate(RUN, {"formula": "", "molecule": "Caffeine"})
            assert x["err"] and x["err"]["key"] == "err.flow.msn.molecule", x["err"]
        step("a molecule name without the ion asks for the ion", no_ion)
        def memory():
            pg.evaluate("window.__f = window.__f || window.fetch; window.fetch = (u, i) => String(u).includes('api/msn_grafo') ? Promise.resolve(new Response(JSON.stringify({ error: 'x', error_key: 'error.memoryBudget', params: {} }))) : window.__f(u, i); null")
            try: x = pg.evaluate(RUN, {"formula": ION})
            finally: pg.evaluate("window.fetch = window.__f; null")
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
        def cache():
            count = "window.__n = 0; window.__g = window.__g || window.fetch; window.fetch = (u, i) => { if (String(u).includes('api/msn_grafo')) window.__n++; return window.__g(u, i); }; null"
            states = "(async () => JSON.parse(await (await fetch('api/flow/sched/nodes', { method: 'POST', body: '{}' })).text()).map(n => n.instance + ':' + n.state))()"
            a = pg.evaluate(RUN, {"formula": ION, "ppm_formula": 7})
            assert a["err"] is None, a["err"]
            pg.evaluate(count)
            try:
                other = pg.evaluate(RUN, {"formula": ION, "ppm_formula": 9})
                assert other["err"] is None, other["err"]
                made = pg.evaluate("window.__n")
                assert made >= 3, made
                pg.evaluate("window.__n = 0")
                back = pg.evaluate(RUN, {"formula": ION, "ppm_formula": 7})
                assert back["err"] is None, back["err"]
                assert pg.evaluate("window.__n") == 0, "the engine was asked again for a result kept in OPFS"
                st = pg.evaluate(states)
                assert all(s.endswith(":reused") for s in st if not s.startswith("ixmix")), st
                assert [mask(f["text"]) for f in back["files"]] == [mask(f["text"]) for f in a["files"]], "the cached texts differ from the first run"
            finally: pg.evaluate("window.fetch = window.__g; null")
            keep = pg.evaluate("""(async () => { const d = await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('flow')).getDirectoryHandle('msn'); const o = []; for await (const [n] of d.entries()) o.push(n); return o })()""")
            assert "index.json" in keep and len([n for n in keep if n.endswith(".json") and n != "index.json"]) >= 6, keep
        step("a result kept in OPFS is taken from there when the plan returns to the same parameters (no request to the engine, job reused)", cache)
        def library_key():
            k = pg.evaluate("""async () => {
              const put = async t => { const w = await (await (await (await navigator.storage.getDirectory()).getFileHandle('msnlib.json', { create: true })).createWritable()); await w.write(t); await w.close() };
              const spec = { key: 'k1' }, inst = { step: 'spectra.match@1' }, other = { step: 'msn.table_export@1' };
              await put('{"a":1}'); const a = await FLOW.liveJobs.msnKey(spec, inst), t1 = await FLOW.liveJobs.msnKey(spec, other);
              await put('{"a":2}'); const b = await FLOW.liveJobs.msnKey(spec, inst), t2 = await FLOW.liveJobs.msnKey(spec, other);
              await put(''); return { same: a === b, table: t1 === t2 } }""")
            assert not k["same"], "a change of the library of the user keeps the key of the search"
            assert k["table"], "the library of the user enters the key of a step that does not read it"
        step("the key of the library search follows the library of the user, the key of the other steps does not", library_key)
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
