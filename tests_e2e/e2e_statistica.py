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

# a time series for the PCA: eight samples (three replicates at t0), a blank and six ions
PCD = HERE / "shots" / "stat_pca_tmp"; PCD.mkdir(parents=True, exist_ok=True)
PC_IONS = [(301.1000, 3.0), (352.2000, 4.0), (410.3000, 5.0), (463.1500, 6.0), (518.2500, 7.0), (577.3500, 8.0)]       # growing, decaying, growing, contaminant, growing, rare
def pc_amp(ion, t, rep):
    k = [1e5 * (1 + t / 15), 6e5 * (1 - t / 120), 2e5 * (1 + t / 30), 8e5, 3e5 * (1 + t / 20), 9e4 if (t == 60 and rep == "a") else 0][ion]
    return k * (1 + 0.04 * (rep == "b") - 0.03 * (rep == "c"))
for t0, rep in ((0, "a"), (0, "b"), (0, "c"), (30, "a"), (30, "b"), (60, "a"), (60, "b"), (90, "a")):
    ds.hr_series(PCD / f"PCA_t{t0}_{rep}.mzML", np.random.default_rng(t0 * 7 + ord(rep)), [(m, rt, 0.06, pc_amp(i, t0, rep)) for i, (m, rt) in enumerate(PC_IONS)])
ds.hr_series(PCD / "PCA_Blank_1.mzML", np.random.default_rng(5), [(PC_IONS[3][0], PC_IONS[3][1], 0.06, 8e5)])

def sections(pg):
    drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec")
    return pg.evaluate("[...document.querySelectorAll('#sb-panel-stat details.stat-sec')].map(d=>d.dataset.s)")

r = Run(port=8932, wd="/tmp/wdstat")
try:
    with sync_playwright() as p:
        pg = r.page(p, qaqc=True)
        load(pg, [D / "S_t0.mzML", D / "S_t30.mzML", D / "S_t60.mzML", D / "Blank_1.mzML"], 4)
        def lr_sections():
            pg.click("#tb-more"); pg.wait_for_selector("#stmenu")
            assert "Statistica" in pg.inner_text("#stmenu"), "the panel is reached from «Altro» (the toolbar has no room for one more button)"
            pg.click("#stmenu >> text=Statistica"); pg.wait_for_selector("#sb-panel-stat details.stat-sec")
            assert sections(pg)[0] == "meta", "metadata first"
            assert any(c["id"] == "stat" for c in pg.evaluate("BARRA.commands.map(c=>({id:c.id}))")), "and from the palette"
        step("«Altro» opens the Statistica panel; metadata is the first section", lr_sections)
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
        pg = r2.page(p, qaqc=True)
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
        def hr_table():
            assert sections(pg) == ["meta", "profile", "features", "blank", "reference", "pca"], sections(pg)
            pg.fill("#feat-list", "305.1234; 6.0\n250.0000; 4.0\nabc; 1"); pg.click("#feat-go")
            assert "1" in pg.inner_text("#feat-msg") and pg.evaluate("TABELLA.table") is None, "an invalid line is refused with its number"
            pg.fill("#feat-list", "305.1234; 6.0\n250.0000; 4.0"); pg.click("#feat-go")
            pg.wait_for_selector("#feat-tab tbody tr", timeout=60000)
            tb = pg.evaluate("TABELLA.table")
            assert len(tb["features"]) == 2 and tb["files"] == pg.evaluate("CAMPIONI.files().map(f=>f.file)")
            a = tb["features"][0]["areas"]
            assert a["HR_t60.mzML"] > a["HR_t30.mzML"] > a["HR_t0.mzML"] > 0 and a["HR_Blank_1.mzML"] == 0, a
            assert tb["features"][1]["areas"]["HR_Blank_1.mzML"] > 1e3, "the internal standard is also in the blank"
        step("HR: feature table measured from a list of ions", hr_table)
        def hr_blank():
            pg.fill("#blank-f", "3"); pg.fill("#blank-n", "1"); pg.click("#blank-go")
            pg.wait_for_selector("#blank-kept", timeout=30000)
            r = pg.evaluate("BIANCO.result.r")
            assert r["kept"] == [0] and r["discarded"] == [1], r                 # the ion that is in the blank at the level of the samples goes to the discarded list
            assert pg.locator("#blank-kept tbody tr").count() == 1 and pg.locator("#blank-disc tbody tr").count() == 1, "both lists are visible"
            pg.fill("#blank-f", "0.1"); pg.click("#blank-go"); pg.wait_for_function("BIANCO.result.r.factor===0.1")
            assert pg.evaluate("BIANCO.result.r.kept.length") == 2
            pg.select_option("#blank-s", "mean"); pg.fill("#blank-n", "4"); pg.click("#blank-go"); pg.wait_for_function("BIANCO.result.r.min_detect===4")
            assert pg.evaluate("BIANCO.result.r.kept.length") == 0, "four detections are impossible with three samples"
            pg.click("#blank-disc tbody tr, #blank-kept tbody tr") if pg.locator("#blank-disc tbody tr").count() else None
        step("HR: blank subtraction with the factor, the level and the minimum chosen by the user; two lists", hr_blank)
        def hr_import():
            names = pg.evaluate("CAMPIONI.files().map(f=>f.file)")
            val = {"HR_t0.mzML": 10, "HR_t30.mzML": 20, "HR_t60.mzML": 30, "HR_Blank_1.mzML": 0}
            csv = "row ID,row m/z,row retention time," + ",".join(n + " Peak area" for n in names) + "\n1,305.1234,6.0," + ",".join(str(val[n]) for n in names) + "\n"
            n = pg.evaluate("(tx)=>{TABELLA.fromText(tx);return TABELLA.table.features.length}", csv)
            assert n == 1 and pg.evaluate("TABELLA.table.source") == "file"
            assert pg.evaluate("TABELLA.matrix()") == [[val[x] for x in names]], "the areas of the table, in the order of the files"
        step("HR: an mzmine table is imported with its areas", hr_import)
        def hr_pca_off():
            drawer(pg, "stat"); pg.wait_for_selector("#stat-pca")
            assert pg.locator("#pca-form").count() == 0 and "5" in pg.inner_text("#stat-pca"), "three samples: the PCA is off, with the reason"
        step("HR: the PCA is off below five samples", hr_pca_off)
finally:
    r2.close()

r3 = Run(port=8934, wd="/tmp/wdstatpca")
try:
    with sync_playwright() as p:
        pg = r3.page(p, qaqc=True)
        load(pg, sorted(PCD.glob("*.mzML")), 9)
        drawer(pg, "stat"); pg.wait_for_selector("#feat-list")
        def pca_table():
            pg.fill("#feat-list", "\n".join(f"{m}; {rt}" for m, rt in PC_IONS)); pg.click("#feat-go")
            pg.wait_for_selector("#feat-tab tbody tr", timeout=120000)
            pg.fill("#blank-f", "3"); pg.fill("#blank-n", "1"); pg.click("#blank-go"); pg.wait_for_selector("#blank-kept", timeout=30000)
            r = pg.evaluate("BIANCO.result.r")
            assert 3 in r["discarded"] and 3 not in r["kept"], "the contaminant of the blank is discarded"
        step("HR: table of six ions measured in nine files, blank subtraction", pca_table)
        def pca_run():
            pg.fill("#pca-min", "5000"); pg.fill("#pca-k", "3"); pg.click("#pca-go"); pg.wait_for_selector("#pca-scores", timeout=60000)
            r = pg.evaluate("PCA.result.r")
            assert r["n_features"] == 4 and [s["n"] for s in r["steps"]] == [6, 5, 5, 4], r["steps"]       # table, above the blank, above the threshold, present in three samples
            assert r["scaling"] == "pareto" and r["transform"] == "log" and r["n_samples"] == 8 and 0.5 < r["explained"][0] <= 1
            assert set(r["ellipses"]) and len(r["ellipses"]) == 1, "only the three replicates of t0 have an ellipse"
            txt = pg.inner_text("#pca-res")
            assert "1/5" in txt and "%" in txt, "the imputation is declared and the variance is shown"
            assert "PC1" in pg.text_content("#pca-scores") and "%" in pg.text_content("#pca-scores"), "variance explained on the axes"
            assert pg.locator("#pca-scores polyline").count() == 1 and pg.locator("#pca-scores polygon").count() == 1, "trajectory and one ellipse"
            order = pg.evaluate("(()=>{const r=PCA.result.r,F=PCA.result.files;return r.samples.map((s,i)=>[F[s].time,r.scores[i][0]]).sort((a,b)=>a[0]-b[0]||a[1]-b[1])})()")
            means = {}
            for t_, v in order: means.setdefault(t_, []).append(v)
            seq = [sum(v) / len(v) for k_, v in sorted(means.items())]
            d = [b - a for a, b in zip(seq, seq[1:])]
            assert all(x > 0 for x in d) or all(x < 0 for x in d), seq
            assert pg.locator("#pca-loadings circle[data-i]").count() == 4
            assert pg.evaluate("PCA.result.r.stability.length") == 4
            assert pg.locator("#sb-panel-stat .cmp-grad").count() >= 1, "gradient legend"
        step("HR: PCA with filters, declared imputation, variance on the axes, trajectory, ellipse of the replicates", pca_run)
        def pca_click():
            pg.click("#pca-tab tbody tr >> nth=0"); ready(pg); pg.wait_for_timeout(800)
            ft = pg.evaluate("PCA.result.feats[+document.querySelector('#pca-tab tbody tr').dataset.i]")
            assert abs(pg.evaluate("PROFILO.result.ion.mz") - ft["mz"]) < 1e-6, "the profile of the ion is shown"
            assert pg.evaluate("E.panels.some(p=>p.pcaIon&&p.traces[0]&&Math.abs(p.traces[0].mz-%s)<1e-3)" % ft["mz"]), "the XIC panel of the ion is open"
            assert pg.locator("#pca-loadings circle[stroke='var(--ink)']").count() == 1, "the loading is highlighted"
        step("HR: a click on a loading shows the XIC and the time profile", pca_click)
        def pca_help():
            pg.click("#pca-help"); assert pg.get_attribute("#pca-help", "aria-expanded") == "true" and pg.is_visible("#pca-help-text")
            pg.select_option("#pca-sc", "auto"); assert pg.is_visible("#pca-scnote"), "the note on auto scaling"
            pg.click("#pca-go"); pg.wait_for_function("PCA.result.r.scaling==='auto'", timeout=30000)
        step("HR: the «?» explains the arch effect; auto scaling carries its warning", pca_help)
finally:
    r3.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs + r2.errs + r3.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
