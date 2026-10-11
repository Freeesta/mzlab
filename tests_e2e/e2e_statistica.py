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

# high resolution series: ion A grows with time, the internal standard (IS) does not; the blank has only the IS
HRD = HERE / "shots" / "stat_hr_tmp"; HRD.mkdir(parents=True, exist_ok=True)
A_MZ, IS_MZ = 305.1234, 250.0000
for name, amp_a, amp_is in (("HR_t0", 1e5, 5e5), ("HR_t30", 4e5, 1e6), ("HR_t60", 8e5, 5e5), ("HR_Blank_1", 0, 5e5)):
    ds.hr_series(HRD / (name + ".mzML"), np.random.default_rng(len(name) + int(amp_a % 97)), [(A_MZ, 6.0, 0.06, amp_a), (IS_MZ, 4.0, 0.06, amp_is)])

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
        def lr_profile():
            assert sections(pg) == ["meta", "profile"], sections(pg)           # low resolution: only the metadata and the profile
            assert pg.locator("#prof-istd").count() == 0, "no normalization in low resolution"
            pg.evaluate("CAMPIONI.apply([{file:'S_t60.mzML',type:'sample',time:60}])")
            pg.fill("#prof-mz", "391.3"); pg.fill("#prof-rt", "18.3"); pg.click("#prof-go")
            pg.wait_for_selector("#prof-res svg.prof-chart", timeout=60000)
            res = pg.evaluate("PROFILO.result")
            pts = [res["rows"][i] for i in res["series"]]
            assert [p["file"] for p in pts] == ["S_t0.mzML", "S_t30.mzML", "S_t60.mzML"], pts            # in time order, the blank has no time
            assert all(p["value"] > 1e4 and abs(p["rt"] - 18.3) < 0.2 for p in pts), pts
            assert pg.locator("#prof-tab tbody tr").count() == 4 and pg.locator("#prof-res circle, #prof-res path[fill]").count() >= 3
            assert pg.locator("#prof-res polyline").count() == 1, "points joined by a line"
            assert pg.evaluate("PROFILO.tsv(PROFILO.result).split('\\n')[0].split('\\t').length") >= 6
            pg.select_option("#prof-metric", "height"); pg.click("#prof-go"); pg.wait_for_timeout(500); ready(pg)
            assert pg.evaluate("PROFILO.result.metric") == "height"
        step("LR: profile of an ion chosen by the student, time order, line, table; no normalization, no ion list", lr_profile)

finally:
    r.close()

r2 = Run(port=8933, wd="/tmp/wdstathr")
try:
    with sync_playwright() as p:
        pg = r2.page(p)
        load(pg, sorted(HRD.glob("*.mzML")), 4)
        def hr_profile():
            assert pg.evaluate("BANCO.on()"), "high resolution"
            assert sections(pg)[:2] == ["meta", "profile"], sections(pg)
            pg.evaluate("QAQC.addIon('250.0000','4.0','IS','is')"); drawer(pg, "file"); drawer(pg, "stat"); ready(pg)
            pg.wait_for_selector("#prof-istd option[value='1']", state="attached")
            pg.select_option("#prof-istd", "1"); pg.fill("#prof-mz", "305.1234"); pg.fill("#prof-rt", "6.0"); pg.click("#prof-go")
            pg.wait_for_selector("#prof-res svg.prof-chart", timeout=60000)
            res = pg.evaluate("PROFILO.result")
            pts = [res["rows"][i] for i in res["series"]]
            assert [round(p["norm"], 1) for p in pts] == [0.2, 0.4, 1.6], pts
            assert res["normalized"] and pg.locator("#prof-tab thead th").count() == 7
            assert pg.evaluate("PROFILO.result.rows.find(r=>r.file==='HR_Blank_1.mzML').value") is None, "the blank has no peak of the ion"
        step("HR: profile of an ion divided by the internal standard of the same file", hr_profile)
finally:
    r2.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs + r2.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
