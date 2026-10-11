"""Statistica panel: sample metadata (low and high resolution), time profile, blank subtraction, exploratory PCA (high resolution only)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_statistica.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))

D = HERE / "shots" / "stat_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.full_scan(D / "S_t0.mzML", 0, np.random.default_rng(1)); ds.full_scan(D / "S_t30.mzML", 30, np.random.default_rng(2)); ds.full_scan(D / "S_t60.mzML", 60, np.random.default_rng(4))
rng = np.random.default_rng(3)
ds._write(D / "Blank_1.mzML", [ds._spec(i, 0.5 + i * 0.0175, list(rng.uniform(120, 480, 45)), list(rng.lognormal(np.log(4e3), 0.9, 45))) for i in range(1111)], [])

def load(pg, files, n):
    pg.set_input_files("#pick", [str(f) for f in files])
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={len(files)}", timeout=120000); pg.wait_for_timeout(1500)
    if pg.is_visible("#opbtn"): pg.click("#opbtn")
    pg.wait_for_function(f"typeof E!=='undefined' && E.files.length>={n}", timeout=60000)
    ready(pg)

def sections(pg):
    drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec")
    return pg.evaluate("[...document.querySelectorAll('#sb-panel-stat details.stat-sec')].map(d=>d.dataset.s)")

r = Run(port=8932, wd="/tmp/wdstat")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "S_t0.mzML", D / "S_t30.mzML", D / "S_t60.mzML", D / "Blank_1.mzML"], 4)
        def lr_sections():
            assert pg.get_attribute("#stbar [data-cmd=stat]", "aria-label") or pg.inner_text("#stbar [data-cmd=stat]")
            assert sections(pg)[0] == "meta", "metadata first"
        step("toolbar button «Statistica» opens the panel; metadata is the first section", lr_sections)
        def meta_table():
            rows = pg.evaluate("[...document.querySelectorAll('#cmp-tab tbody tr')].map(tr=>[tr.querySelector('[data-m=type]').value, tr.querySelector('[data-m=time]').value])")
            d = dict(zip(pg.evaluate("E.files.map(f=>f.file)"), rows))
            assert d["S_t30.mzML"] == ["sample", "30"] and d["Blank_1.mzML"][0] == "blank", d          # pre-filled from the names (project.py)
        step("table pre-filled from the file names", meta_table)
        def edit():
            sel = "#cmp-tab [data-f='S_t60.mzML']"
            pg.select_option(sel + "[data-m=type]", "qc"); ready(pg)
            assert pg.evaluate("QAQC.roleOf(E.files.find(f=>f.file==='S_t60.mzML'))") == "qc"
            assert pg.evaluate("E.files.find(f=>f.file==='S_t60.mzML').type") == "sample"
            pg.fill(sel + "[data-m=time]", "1,5"); pg.dispatch_event(sel + "[data-m=time]", "change"); ready(pg)
            assert pg.evaluate("E.files.find(f=>f.file==='S_t60.mzML').time") == 1.5
            pg.fill("#cmp-tab [data-f='S_t0.mzML'][data-m=rep]", "R1"); pg.dispatch_event("#cmp-tab [data-f='S_t0.mzML'][data-m=rep]", "change"); ready(pg)
            assert pg.evaluate("E.files.find(f=>f.file==='S_t0.mzML').rep") == "R1"
            j = pg.evaluate("fetch('api/session').then(r=>r.json())")["session"]
            m = {x["file"]: x for x in j}
            assert m["S_t60.mzML"]["time"] == 1.5 and m["S_t0.mzML"]["rep"] == "R1", "server copy updated"
            pg.fill(sel + "[data-m=time]", "abc"); pg.dispatch_event(sel + "[data-m=time]", "change")
            assert pg.is_visible("#cmp-msg.bad"), "a non-number is refused with a message"
            assert pg.evaluate("E.files.find(f=>f.file==='S_t60.mzML').time") == 1.5
        step("type, time (decimal comma), replicate edit the files, QAQC roles and the server copy; a non-number is refused", edit)
        def tsv():
            t = pg.evaluate("CAMPIONI.toTsv(CAMPIONI.rows(), tunitGet())").splitlines()
            assert t[0] == "filename\tsample_type\ttime\ttime_unit\tconcentration\tconcentration_unit\treplicate" and len(t) == 5, t
            body = "\n".join(l.replace("S_t30.mzML\tsample\t30\tmin", "S_t30.mzML\tblank\t2\th") for l in t) + "\n"
            n = pg.evaluate("(tx)=>CAMPIONI.importTsv(tx).changed", body)
            assert n == 4
            f = pg.evaluate("(()=>{const f=E.files.find(f=>f.file==='S_t30.mzML');return [f.type,f.time]})()")
            assert f == ["blank", 120], f
            assert pg.evaluate("CAMPIONI.importTsv('filename\\tsample_type\\nnofile.mzML\\tblank\\n').unknown.length") == 1
        step("TSV export in the mzmine columns; import converts the unit and reports unknown files", tsv)
        def gradient():
            pg.evaluate("CAMPIONI.apply([{file:'S_t30.mzML',type:'sample',time:30}])"); ready(pg)
            drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat .cmp-grad")
            assert pg.get_attribute("#sb-panel-stat .cmp-grad", "aria-label")
            cols = pg.evaluate("E.files.filter(f=>f.type==='sample'&&f.time!=null).sort((a,b)=>a.time-b.time).map(f=>f.color)")
            assert len(set(cols)) == len(cols) >= 3, cols
        step("time gradient with a legend", gradient)
        def chart_legend():
            pg.evaluate("(()=>{E.files.forEach(f=>{f.vis=true});redrawAll()})()"); ready(pg); pg.wait_for_timeout(800)
            assert pg.evaluate("E.panels.some(p=>p.leg&&p.leg.querySelector('.cmp-grad'))"), "the legend of a chart with the files of a time series carries the gradient bar"
        step("chart legend carries the gradient bar", chart_legend)
        def lr_only_meta():
            assert sections(pg) == ["meta"], sections(pg)                      # low resolution: PR 1 only has the metadata section
        step("low resolution shows the metadata section", lr_only_meta)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
