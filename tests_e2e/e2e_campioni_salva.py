"""One writer and one saving of roles and times (campioni/2): an edit in the Data view survives a reload (role, subtype, time, confirmation), a notebook v1
with QC and dark roles of qqq.qaqc is migrated without loss, and a .mzflow saved and opened again keeps the SHA-256 of its `campioni` operation.
Synthetic files with invented names."""
import sys, os, json, shutil, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_campioni_salva.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
NAMES = ["X_t000min", "X_t005min", "X_t030min", "X_t060min", "X_dark-adsorption", "X_blank"]
tmp = Path(tempfile.mkdtemp(prefix="cs_"))
paths = []
for k, n in enumerate(NAMES):
    p = tmp / f"{n}.mzML"; synth.write_full(p, 5 * k, blank=n.endswith("blank"), seed=k); paths.append(str(p))
SNAP = "E.files.filter(f=>!f.gone).map(f=>[f.file, CAMPIONI.kindOf(f), f.sub||null, f.time??null, f.tconf!==false])"
r = Run(port=8830 + 150, wd="/tmp/wd_campioni_salva")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, paths)
        pg.click("#opbtn"); ready(pg)
        snap = {}
        def edit_and_reload():
            pg.evaluate("""() => CAMPIONI.apply([{file:'X_blank.mzML', role:'blank', sub:'procedure'}, {file:'X_t005min.mzML', time:7, tconf:true}, {file:'X_t030min.mzML', role:'qc'}, {file:'X_t060min.mzML', time:61, tconf:false}])""")
            snap["a"] = pg.evaluate(SNAP)
            kinds = {x[0]: x for x in snap["a"]}
            assert kinds["X_blank.mzML"][1:3] == ["blank", "procedure"] and kinds["X_t030min.mzML"][1] == "qc" and kinds["X_t005min.mzML"][3] == 7 and kinds["X_t060min.mzML"][4] is False, kinds
            pg.evaluate("uiSave(true)")
            attendi(pg, "fetch('api/notebook').then(r => r.json()).then(j => (j.session || []).some(x => x.file === 'X_t005min.mzML' && x.tempo_min === 7))")
            pg.reload(); pg.wait_for_function("typeof E !== 'undefined' && E.files.length === 6", timeout=90000); ready(pg)
            after = pg.evaluate(SNAP)
            assert after == snap["a"], (after, snap["a"])
        step("an edit survives a reload: role, subtype, time, confirmation", edit_and_reload)
        def one_writer():
            nb = pg.evaluate("NB.session")
            assert nb and all(x.get("tipo") and "tempo_min" in x for x in nb) and pg.evaluate("NB._v") == 2, nb[:1]
            assert pg.evaluate("(()=>{try{return Object.keys(JSON.parse(localStorage.getItem('qqq.qaqc')||'{}').roles||{}).length}catch(e){return 0}})()") == 0, "qqq.qaqc.roles is not written any more"
            assert pg.evaluate("QAQC.roleOf(E.files.find(f=>f.file==='X_t030min.mzML'))") == "qc"
            pg.evaluate("CAMPIONI.apply([{file:'X_t030min.mzML', role:'dark'}])")
            assert pg.evaluate("QAQC.roleOf(E.files.find(f=>f.file==='X_t030min.mzML'))") == "dark" and pg.evaluate("E.files.find(f=>f.file==='X_t030min.mzML').type") == "sample"
            pg.evaluate("CAMPIONI.apply([{file:'X_t030min.mzML', role:'qc'}])")
        step("NB.session is campioni/2, qqq.qaqc.roles is empty, the QA/QC follows the table", one_writer)
        def migrate_v1():
            pg.evaluate("() => { window.uiSave = () => {}; window.nbSave = () => {}; }")           # the page leaving must not overwrite the notebook written below
            v1 = {"_v": 1, "session": [{"file": "X_t000min.mzML", "time": 0, "type": "sample"}, {"file": "X_t005min.mzML", "time": 5, "type": "sample"}, {"file": "X_t030min.mzML", "time": 30, "type": "sample"},
                  {"file": "X_t060min.mzML", "time": 60, "type": "sample"}, {"file": "X_dark-adsorption.mzML", "time": 15, "type": "sample"}, {"file": "X_blank.mzML", "type": "blank"}]}
            pg.evaluate("""async (v1) => { await fetch('api/notebook', {method: 'POST', body: JSON.stringify(v1)});
                localStorage.setItem('qqq.qaqc', JSON.stringify({roles: {'X_t030min.mzML': 'qc', 'X_dark-adsorption.mzML': 'dark'}, ions: [], thr: 3, win: 0.5, ratio: {text: '', rt: ''}, seq: 1})); }""", v1)
            pg.reload(); pg.wait_for_function("typeof E !== 'undefined' && E.files.length === 6", timeout=90000); ready(pg)
            got = {x[0]: x for x in pg.evaluate(SNAP)}
            assert got["X_t000min.mzML"][1] == "t0" and got["X_t030min.mzML"][1] == "qc" and got["X_dark-adsorption.mzML"][1] == "dark" and got["X_blank.mzML"][1] == "blank", got
            assert got["X_t005min.mzML"][3] == 5 and got["X_t060min.mzML"][3] == 60 and got["X_t005min.mzML"][4] is True, got
            nb = pg.evaluate("NB.session"); assert all(x.get("tipo") for x in nb) and pg.evaluate("NB._v") == 2, nb[:1]
        step("a notebook v1 with QC and dark roles is migrated without loss", migrate_v1)
        def mzflow():
            pg.evaluate("CAMPIONI.apply([{file:'X_blank.mzML', role:'blank', sub:'solvent'}, {file:'X_t030min.mzML', role:'qc'}, {file:'X_t005min.mzML', time:6, tconf:true}])")
            with pg.expect_download() as d: pg.evaluate("() => WORKFLOW.export()")
            wf = json.load(open(d.value.path(), encoding="utf-8"))
            ops = [o for o in wf["operazioni"] if o.get("op") == "campioni"]
            assert len(ops) == 1 and ops[0]["schema"] == "campioni/2" and len(ops[0]["sha256"]) == 64 and len(ops[0]["campioni"]) == 6, ops
            before = pg.evaluate(SNAP)
            pg.evaluate("CAMPIONI.apply([{file:'X_blank.mzML', role:'sample'}, {file:'X_t030min.mzML', role:'sample'}, {file:'X_t005min.mzML', time:9, tconf:false}])")
            assert pg.evaluate(SNAP) != before
            assert pg.evaluate("(wf) => WORKFLOW.applyWorkflow(wf, 't.mzflow')", wf) is True
            assert pg.evaluate(SNAP) == before, (pg.evaluate(SNAP), before)
            with pg.expect_download() as d2: pg.evaluate("() => WORKFLOW.export()")
            wf2 = json.load(open(d2.value.path(), encoding="utf-8"))
            ops2 = [o for o in wf2["operazioni"] if o.get("op") == "campioni"]
            assert len(ops2) == 1 and ops2[0]["sha256"] == ops[0]["sha256"] and ops2[0]["campioni"] == ops[0]["campioni"], (ops2[0]["sha256"], ops[0]["sha256"])
        step(".mzflow saved and opened again: the SHA-256 of the operation campioni does not change", mzflow)
        def mzflow_old():
            with pg.expect_download() as d: pg.evaluate("() => WORKFLOW.export()")
            wf = json.load(open(d.value.path(), encoding="utf-8"))
            before = pg.evaluate(SNAP)
            wf["operazioni"] = [o for o in wf["operazioni"] if o.get("op") != "campioni"]
            for f in wf["file"]: f.pop("ruolo", None)                                              # a file of the old program: base type and qaqc.ruoli only
            pg.evaluate("CAMPIONI.apply([{file:'X_blank.mzML', role:'sample'}, {file:'X_t030min.mzML', role:'sample'}])")
            assert pg.evaluate("(wf) => WORKFLOW.applyWorkflow(wf, 'old.mzflow')", wf) is True
            got = {x[0]: x for x in pg.evaluate(SNAP)}
            assert got["X_blank.mzML"][1] == "blank" and got["X_t030min.mzML"][1] == "qc", got
        step("an old .mzflow is read through da_legacy", mzflow_old)
finally:
    r.close(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
