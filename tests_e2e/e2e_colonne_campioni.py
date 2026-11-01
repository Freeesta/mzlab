"""Extra columns of the loading table: a preset column and a custom one are added from the «Columns» dialog, values are typed in the cells, a reload keeps the definitions and the values,
and the TSV of the sample table (the export of the Statistica panel) carries the columns. Synthetic files."""
import sys, os, shutil, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
NAMES = ["X_t000min", "X_t005min"]
tmp = Path(tempfile.mkdtemp(prefix="cols_"))
paths = []
for k, n in enumerate(NAMES):
    p = tmp / f"{n}.mzML"; synth.write_full(p, 5 * k, seed=k); paths.append(str(p))
r = Run(port=8830 + 150, wd="/tmp/wd_colonne")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, paths)
        pg.wait_for_function("RUOLI.busy() === 0 && ST.files.length >= 2 && ST.files.every(f => f.prop)", timeout=30000)
        lib.show_table(pg)
        pg.wait_for_selector("#colsbtn", timeout=15000)
        def add():
            pg.click("#colsbtn"); pg.wait_for_selector("#colsdlg[open]")
            pg.check("#colsdlg [data-cc=dil]")
            pg.fill("#colsnew", "Pump"); pg.select_option("#colstype", "number"); pg.fill("#colsunit", "bar"); pg.click("#colsadd")
            pg.click("#colsclose")
            heads = pg.eval_on_selector_all("#flist thead th", "e => e.map(x => x.textContent.trim())")
            dil = pg.evaluate("I18N.t('load.cols.p.dil')")
            assert dil in heads and "Pump (bar)" in heads, heads
        step("a preset and a custom column are added", add)
        def fill():
            sel = "#flist tr:has(:text-is('X_t000min')) input[data-xf]"
            inputs = pg.query_selector_all(sel); assert len(inputs) == 2
            inputs[0].fill("10"); inputs[0].dispatch_event("change")
            inputs[1].fill("abc"); inputs[1].dispatch_event("change")
            assert inputs[1].get_attribute("aria-invalid") == "true", "a text in a number column is flagged"
            inputs[1].fill("2,5"); inputs[1].dispatch_event("change")
            assert pg.evaluate("Object.values(NB.sampleEx['X_t000min.mzML']).join('|')") == "10|2.5"
        step("values are typed and checked", fill)
        def export():
            tx = pg.evaluate("COLS.tsv(CAMPIONI.toTsv(CAMPIONI.rows(), 'min'))")
            lines = [l for l in tx.split("\n") if l]
            dil = pg.evaluate("I18N.t('load.cols.p.dil')")
            assert lines[0].endswith(f"\t{dil}\tPump (bar)"), lines[0]
            row = [l for l in lines if l.startswith("X_t000min.mzML")][0]
            assert row.endswith("\t10\t2.5"), row
        step("the TSV carries the columns", export)
        def reload():
            pg.evaluate("nbSave()"); pg.reload(); pg.wait_for_function("typeof NB !== 'undefined' && NB.sampleCols && NB.sampleCols.on.length === 2", timeout=30000)
            assert pg.evaluate("JSON.stringify(NB.sampleCols.on.length)") == "2"
            assert pg.evaluate("NB.sampleEx['X_t000min.mzML'] && Object.keys(NB.sampleEx['X_t000min.mzML']).length") == 2
        step("a reload keeps the definitions and the values", reload)
finally:
    r.close(); shutil.rmtree(tmp, ignore_errors=True)
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
