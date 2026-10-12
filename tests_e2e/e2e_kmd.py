"""Statistica, «Kendrick mass defect» (high resolution only): the section exists in HR and not in LR, a PFAS-like series of CF2 homologues shows as ONE candidate
series with its evidence, unrelated ions stay out, a click opens XIC and profile, the plot has its table, another unit regroups, the saved parameters come back."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_kmd.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
def load(pg, files, n):
    pg.set_input_files("#pick", [str(f) for f in files])
    pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={len(files)}", timeout=120000); pg.wait_for_timeout(1500)
    if pg.is_visible("#opbtn"): pg.click("#opbtn")
    pg.wait_for_function(f"typeof E!=='undefined' && E.files.length>={n}", timeout=60000)
    ready(pg)
def sections(pg):
    drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec")
    return pg.evaluate("[...document.querySelectorAll('#sb-panel-stat details.stat-sec')].map(d=>d.dataset.s)")

LRD = HERE / "shots" / "kmd_lr_tmp"; LRD.mkdir(parents=True, exist_ok=True)
ds.full_scan(LRD / "S_t0.mzML", 0, np.random.default_rng(1)); ds.full_scan(LRD / "S_t30.mzML", 30, np.random.default_rng(2))
HRD = HERE / "shots" / "kmd_hr_tmp"; HRD.mkdir(parents=True, exist_ok=True)
NAMES = ["HR_t0", "HR_t30"]
PF = 305.1234
for i, n in enumerate(NAMES): ds.hr_series(HRD / (n + ".mzML"), np.random.default_rng(10 + i), [(PF, 6.0, 0.06, 4e5)])
PFCA = [212.97918, 262.97599, 312.97279, 362.96959, 412.96640, 462.96320, 512.96001]       # [M-H]- of the C4..C10 perfluoroalkyl carboxylic acids
RT = [3.1, 4.6, 5.9, 7.0, 8.0, 8.9, 9.7]
rows = [(i + 1, mz, rt) for i, (mz, rt) in enumerate(zip(PFCA, RT))] + [(20, 301.1234, 6.0), (21, 455.3141, 2.0)]
CSV = "row ID,row m/z,row retention time," + ",".join(n + " Peak area" for n in NAMES) + "\n" + "\n".join(f"{i},{mz},{rt},1000,2000" for i, mz, rt in rows)
r, r2 = Run(port=8947, wd="/tmp/wdkmd"), Run(port=8948, wd="/tmp/wdkmd2")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [LRD / "S_t0.mzML", LRD / "S_t30.mzML"], 2)
        def lr_off():
            assert not pg.evaluate("BANCO.on()"); assert "kmd" not in sections(pg), sections(pg)
        step("LR: the section does not exist", lr_off)
    with sync_playwright() as p:
        pg = r2.page(p)
        load(pg, [HRD / f"{n}.mzML" for n in NAMES], len(NAMES))
        def series():
            assert pg.evaluate("BANCO.on()") and "kmd" in sections(pg), sections(pg)
            pg.evaluate("t => TABELLA.fromText(t)", CSV); pg.wait_for_selector("#kmd-series tbody tr", timeout=30000)
            assert pg.locator("#kmd-series tbody tr").count() == 1, "one candidate series"
            res = pg.evaluate("KMD.result"); s = res["series"][0]
            assert s["members"] == list(range(7)) and s["steps"] == [1] * 6 and s["rho"] > 0.99 and s["spanMDa"] < 1, s
            assert [x["series"] for x in res["rows"][7:]] == [None, None], "unrelated ions stay out"
            assert pg.locator("#kmd-plot g[data-i]").count() == 9 and "7" in pg.inner_text("#kmd-series")
            assert pg.get_attribute("#kmd-plot", "role") == "img" and "CF2" in pg.get_attribute("#kmd-plot", "aria-label")
        step("a CF2 series: one candidate with members, steps and a retention trend; unrelated ions stay out", series)
        def click():
            pg.locator("#kmd-plot g[data-i]").nth(2).dispatch_event("click"); pg.wait_for_function("E.panels.some(p => p.kmdIon)", timeout=30000)
            assert pg.evaluate("KMD.result") is not None and pg.locator("#kmd-tab tr.sel").count() == 1
        step("a click on a point opens the XIC of the ion", click)
        def table():
            pg.evaluate("TDATI.open && 1"); t = pg.evaluate("KMD.plotTable()"); assert len(t["rows"]) == 9 and t["head"][4] == "KMD", t["head"]
        step("the plot has its data table", table)
        def unit():
            pg.select_option("#kmd-unit", "CH2"); pg.click("#kmd-go"); pg.wait_for_function("KMD.params.unit==='CH2'", timeout=10000)
            assert pg.evaluate("KMD.result.series.length") == 0 or pg.evaluate("KMD.result.series[0].members.length") < 7, "another unit does not regroup the CF2 series"
            pg.reload(); pg.wait_for_timeout(1500); assert pg.evaluate("KMD.params.unit") == "CH2", "the parameters come back"
        step("another unit regroups, the parameters are saved", unit)
finally:
    r.close(); r2.close()
r.report(); r2.report()
for s in steps: print(" ", s)
