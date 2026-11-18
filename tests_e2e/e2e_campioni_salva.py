"""One writer and one saving of roles and times (samples/1): an edit in the Data view survives a reload (role, subtype, time, confirmation) and a .mzflow saved and opened again keeps the SHA-256 of its `campioni` operation.
Synthetic files with invented names."""
# covers: mzlab/web/shell.js, mzlab/web/campioni.js, mzlab/web/campioni-doc.js, mzlab/web/campioni-tabella.js, mzlab/web/ruoli-carica.js
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
SNAP = "E.files.filter(f=>!f.gone).map(f=>[f.file, CAMPIONI.kindOf(f), f.time??null, f.tconf!==false])"
r = Run(port=8830 + 150, wd="/tmp/wd_campioni_salva")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, paths)
        pg.click("#opbtn"); ready(pg)
        snap = {}
        def edit_and_reload():
            pg.evaluate("""() => CAMPIONI.apply([{file:'X_blank.mzML', role:'blank'}, {file:'X_t005min.mzML', time:7, tconf:true}, {file:'X_t030min.mzML', role:'qc'}, {file:'X_t060min.mzML', time:61, tconf:false}])""")
            snap["a"] = pg.evaluate(SNAP)
            kinds = {x[0]: x for x in snap["a"]}
            assert kinds["X_blank.mzML"][1] == "blank" and kinds["X_t030min.mzML"][1] == "qc" and kinds["X_t005min.mzML"][2] == 7 and kinds["X_t060min.mzML"][3] is False, kinds
            pg.evaluate("uiSave(true)")
            wait_for_js(pg, "fetch('api/notebook').then(r => r.json()).then(j => (j.session || []).some(x => x.file === 'X_t005min.mzML' && x.time_min === 7))")
            pg.reload(); pg.wait_for_function("typeof E !== 'undefined' && E.files.length === 6", timeout=90000); ready(pg)
            after = pg.evaluate(SNAP)
            assert after == snap["a"], ([x for x in after if x not in snap["a"]], [x for x in snap["a"] if x not in after])
        step("an edit survives a reload: role, time, confirmation", edit_and_reload)
        def one_writer():
            nb = pg.evaluate("NB.session")
            assert nb and all(x.get("kind") and "time_min" in x for x in nb) and pg.evaluate("NB._v") == 2, nb[:1]
            assert pg.evaluate("(()=>{try{return Object.keys(JSON.parse(localStorage.getItem('mzlab.qaqc')||'{}').roles||{}).length}catch(e){return 0}})()") == 0, "mzlab.qaqc.roles is not written any more"
            assert pg.evaluate("QAQC.roleOf(E.files.find(f=>f.file==='X_t030min.mzML'))") == "qc"
            pg.evaluate("CAMPIONI.apply([{file:'X_t030min.mzML', role:'dark'}])")
            assert pg.evaluate("QAQC.roleOf(E.files.find(f=>f.file==='X_t030min.mzML'))") == "dark" and pg.evaluate("E.files.find(f=>f.file==='X_t030min.mzML').type") == "sample"
            pg.evaluate("CAMPIONI.apply([{file:'X_t030min.mzML', role:'qc'}])")
        step("NB.session is samples/1, mzlab.qaqc.roles is empty, the QA/QC follows the table", one_writer)
        def old_documents():
            """A saved project of the old program: role «t0», blank subtypes, a dark with a positive time. It opens as sample at 0, plain blanks and a negative dark."""
            pg.evaluate("() => { window.uiSave = () => {}; window.nbSave = () => {}; }")
            def e(file, kind, time=None, blank_kind=None, k=1):
                return {"compound": "", "concentration": None, "file": file, "istd": False, "note": "", "injection_order": k, "replicate": 1, "series": "", "sha256": "", "time": time,
                        "time_confirmed": time is not None, "time_min": time, "kind": kind, "blank_kind": blank_kind, "concentration_unit": None, "time_unit": "min"}
            old = {"_v": 2, "session": [e("X_t000min.mzML", "t0", 0, k=1), e("X_t005min.mzML", "sample", 5, k=2), e("X_t030min.mzML", "sample", 30, k=3), e("X_t060min.mzML", "sample", 60, k=4),
                                        e("X_dark-adsorption.mzML", "dark", 15, k=5), e("X_blank.mzML", "blank", None, "solvent", k=6)]}
            pg.evaluate("async (d) => { await fetch('api/notebook', {method: 'POST', body: JSON.stringify(d)}); localStorage.removeItem('mzlab.qaqc'); }", old)
            pg.reload(); pg.wait_for_function("typeof E !== 'undefined' && E.files.length === 6", timeout=90000); ready(pg)
            got = {x[0]: x for x in pg.evaluate(SNAP)}
            assert got["X_t000min.mzML"][1] == "sample" and got["X_t000min.mzML"][2] == 0, got
            assert got["X_blank.mzML"][1] == "blank" and pg.evaluate("E.files.every(f => f.sub == null)"), got
            assert got["X_dark-adsorption.mzML"][1] == "dark" and got["X_dark-adsorption.mzML"][2] == -15, got
            assert "-1" not in str(got["X_t005min.mzML"]) and got["X_t005min.mzML"][2] == 5, got
            ent = {x["file"]: x for x in pg.evaluate("CDOC.session(E.files.filter(f => !f.gone))")}
            assert ent["X_t000min.mzML"]["kind"] == "sample" and ent["X_blank.mzML"]["blank_kind"] is None and ent["X_dark-adsorption.mzML"]["time_min"] == -15, ent
            assert pg.evaluate("CAMPIONI.rows().find(r => r.filename === 'X_blank.mzML').sub") is None
            assert "blank_type" not in pg.evaluate("CAMPIONI.COLS"), "the TSV has no blank_type column"
        step("a project with t0, blank:solvent and a positive dark time opens as sample at 0, blank and negative dark", old_documents)
        def mzflow():
            pg.evaluate("CAMPIONI.apply([{file:'X_blank.mzML', role:'blank'}, {file:'X_t030min.mzML', role:'qc'}, {file:'X_t005min.mzML', time:6, tconf:true}, {file:'X_dark-adsorption.mzML', time:40}])")
            assert pg.evaluate("E.files.find(f => f.file === 'X_dark-adsorption.mzML').time") == -40, "a positive time written in a dark is read as negative"
            with pg.expect_download() as d: pg.evaluate("() => WORKFLOW.export()")
            wf = json.load(open(d.value.path(), encoding="utf-8"))
            ops = [o for o in wf["operazioni"] if o.get("op") == "campioni"]
            assert len(ops) == 1 and ops[0]["schema"] == "samples/1" and len(ops[0]["sha256"]) == 64 and len(ops[0]["samples"]) == 6, ops
            before = pg.evaluate(SNAP)
            pg.evaluate("CAMPIONI.apply([{file:'X_blank.mzML', role:'sample'}, {file:'X_t030min.mzML', role:'sample'}, {file:'X_t005min.mzML', time:9, tconf:false}])")
            assert pg.evaluate(SNAP) != before
            assert pg.evaluate("(wf) => WORKFLOW.applyWorkflow(wf, 't.mzflow')", wf) is True
            assert pg.evaluate(SNAP) == before, (pg.evaluate(SNAP), before)
            with pg.expect_download() as d2: pg.evaluate("() => WORKFLOW.export()")
            wf2 = json.load(open(d2.value.path(), encoding="utf-8"))
            ops2 = [o for o in wf2["operazioni"] if o.get("op") == "campioni"]
            assert len(ops2) == 1 and ops2[0]["sha256"] == ops[0]["sha256"] and ops2[0]["samples"] == ops[0]["samples"], (ops2[0]["sha256"], ops[0]["sha256"])
        step(".mzflow saved and opened again: the SHA-256 of the operation campioni does not change", mzflow)
finally:
    r.close(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
