"""Statistica, «Kendrick mass defect» and «PFAS» (high resolution only): the sections exist in HR and not in LR, a PFAS-like series of CF2 homologues shows as ONE candidate
series with its evidence, unrelated ions stay out, a click opens XIC and profile, the plot has its table, another unit regroups, the saved parameters come back.
PFAS section: the candidate formula (with fluorine) and MD/C against m/C of every ion, the diagnostic fragments and losses of the MS2 that selected it (counted as
evidence, never a label), unrelated ions have none, the plot has its table, a click opens the XIC and the MS2 scan."""
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
PFCA = [212.97918, 262.97599, 312.97279, 362.96959, 412.96640, 462.96320, 512.96001]       # [M-H]- of the C4..C10 perfluoroalkyl carboxylic acids
RT = [3.1, 4.6, 5.9, 7.0, 8.0, 8.9, 9.7]
FE, EL = 18.99840316273, 0.00054857990946
CFN = lambda n: n * 12 + (2 * n + 1) * FE + EL                                              # CnF(2n+1)- (anion)
# MS2 that the file contains: precursor -> [(fragment m/z, relative intensity)]; PFOA has the ladder and the loss of CO2, the C6 acid only the ladder start, the third ion is unrelated
MS2 = {PFCA[4]: [(CFN(1), .3), (CFN(2), .4), (CFN(3), 1.), (CFN(4), .5), (PFCA[4] - 43.98983, .8)], PFCA[2]: [(CFN(1), .5), (CFN(2), 1.)], 301.1234: [(101.0233, 1.), (155.5, .4), (210.7, .3)]}
def pfas_dda(path, rng, ions, ms2):
    """Orbitrap-like negative file (all numbers invented): Full Scans with the ions (m/z, RT, sigma, amplitude) and an MS2 of each ion of `ms2` while it is intense."""
    pre, specs, rts, tic, idx = "controllerType=0 controllerNumber=1 scan=", [], [], [], 0
    def spec(rt, level, mz, it, scan_extra, prec=""):
        o = np.argsort(mz); mz = np.array(mz)[o]; it = np.array(it)[o]; b = int(np.argmax(it))
        return (f'<spectrum index="{idx}" id="{pre}{idx + 1}" defaultArrayLength="{len(mz)}">' + ds._cvp("MS:1000579" if level == 1 else "MS:1000580", "scan") + ds._cvp("MS:1000511", "ms level", level)
                + ds._cvp("MS:1000129", "negative scan") + ds._cvp("MS:1000127", "centroid spectrum") + ds._cvp("MS:1000504", "base peak m/z", f"{mz[b]:.6f}", ds._U_MZ)
                + ds._cvp("MS:1000505", "base peak intensity", f"{it[b]:.6g}") + ds._cvp("MS:1000285", "total ion current", f"{float(it.sum()):.6g}")
                + f'<scanList count="1">{ds._cvp("MS:1000795", "no combination")}<scan>' + ds._cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
                + scan_extra + '</scan></scanList>' + prec + '<binaryDataArrayList count="2">' + ds._arr("MS:1000514", "m/z array", mz) + ds._arr("MS:1000515", "intensity array", it) + "</binaryDataArrayList></spectrum>")
    for rt in np.arange(0.5, 12.0, 0.04):
        mz = list(rng.uniform(100, 900, 100)); it = list(rng.lognormal(np.log(3e3), 0.5, 100)); vals = {}
        for m, c, sg, a in ions:
            v = a * float(ds._g(rt, c, sg)) * rng.normal(1, 0.02)
            if v >= 1e3: mz.append(m * (1 + rng.normal(0, 0.8e-6))); it.append(v); vals[m] = v
        sc = ds._cvp("MS:1000800", "mass resolving power", 60000) + ds._cvp("MS:1000512", "filter string", "FTMS - p ESI Full ms [100.0000-900.0000]")
        specs.append(spec(rt, 1, mz, it, sc)); rts.append(rt); tic.append(float(np.sum(it))); ref = idx; idx += 1
        for m, fr in ms2.items():
            if vals.get(m, 0) < 1.5e5: continue
            fm = [f * (1 + rng.normal(0, 0.8e-6)) for f, _ in fr] + list(rng.uniform(60, m, 6)); fi = [r * vals[m] * .2 for _, r in fr] + list(rng.lognormal(np.log(1.5e3), .4, 6))
            sc2 = ds._cvp("MS:1000800", "mass resolving power", 15000) + ds._cvp("MS:1000512", "filter string", f"FTMS - p ESI d Full ms2 {m:.4f}@hcd30.00 [50.0000-{m + 10:.4f}]")
            prec = (f'<precursorList count="1"><precursor spectrumRef="{pre}{ref + 1}"><isolationWindow>' + ds._cvp("MS:1000827", "isolation window target m/z", f"{m:.6f}", ds._U_MZ)
                    + ds._cvp("MS:1000828", "isolation window lower offset", .75, ds._U_MZ) + ds._cvp("MS:1000829", "isolation window upper offset", .75, ds._U_MZ) + "</isolationWindow>"
                    + '<selectedIonList count="1"><selectedIon>' + ds._cvp("MS:1000744", "selected ion m/z", f"{m:.6f}", ds._U_MZ) + ds._cvp("MS:1000041", "charge state", -1) + "</selectedIon></selectedIonList><activation>"
                    + ds._cvp("MS:1000422", "beam-type collision-induced dissociation") + ds._cvp("MS:1000045", "collision energy", 30.0, ' unitCvRef="UO" unitAccession="UO:0000266" unitName="electronvolt"') + "</activation></precursor></precursorList>")
            specs.append(spec(rt + .004, 2, fm, fi, sc2, prec)); idx += 1
    chroms = [ds._chrom(0, "TIC", np.array(rts), np.array(tic))]
    Path(path).write_text(ds._hr_head("exploris", False, Path(path).stem) + f'<spectrumList count="{len(specs)}">' + "".join(specs) + "</spectrumList>"
                          + f'<chromatogramList count="{len(chroms)}">' + "".join(chroms) + "</chromatogramList></run></mzML>", encoding="utf-8")
SER = [(mz, rt, 0.06, 4e5) for mz, rt in zip(PFCA, RT)] + [(301.1234, 6.0, .06, 4e5), (455.3141, 2.0, .06, 4e5)]
for i, n in enumerate(NAMES): pfas_dda(HRD / (n + ".mzML"), np.random.default_rng(10 + i), SER, MS2)
rows = [(i + 1, mz, rt) for i, (mz, rt) in enumerate(zip(PFCA, RT))] + [(20, 301.1234, 6.0), (21, 455.3141, 2.0)]
CSV = "row ID,row m/z,row retention time," + ",".join(n + " Peak area" for n in NAMES) + "\n" + "\n".join(f"{i},{mz},{rt},1000,2000" for i, mz, rt in rows)
r, r2 = Run(port=8947, wd="/tmp/wdkmd"), Run(port=8948, wd="/tmp/wdkmd2")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [LRD / "S_t0.mzML", LRD / "S_t30.mzML"], 2)
        def lr_off():
            assert not pg.evaluate("BANCO.on()"); assert "kmd" not in sections(pg) and "pfas" not in sections(pg), sections(pg)
        step("LR: the section does not exist", lr_off)
    with sync_playwright() as p:
        pg = r2.page(p)
        load(pg, [HRD / f"{n}.mzML" for n in NAMES], len(NAMES))
        def series():
            assert pg.evaluate("BANCO.on()") and "kmd" in sections(pg) and "pfas" in sections(pg), sections(pg)
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
        def pfas():
            pg.evaluate("document.querySelector('#sb-panel-stat details.stat-sec[data-s=pfas]').open = true")
            pg.click("#pfas-go"); pg.wait_for_function("PFAS.result && PFAS.result.rows.length === 9", timeout=120000)
            rows = pg.evaluate("PFAS.result.rows.map(r => ({mz: r.mz, f: r.formula, c: r.c, mdc: r.mdc, ev: r.ms2 ? r.ms2.ev.n : null, fr: r.ms2 ? r.ms2.ev.frags.map(x => x.id) : []}))")
            assert [r["f"] for r in rows[:7]] == [f"C{n}F{2 * n - 1}O2" for n in range(4, 11)], [r["f"] for r in rows]
            assert rows[4]["c"] == 8 and abs(rows[4]["mdc"] - (PFCA[4] - round(PFCA[4])) / 8) < 1e-9
            assert rows[7]["f"] is None and rows[8]["f"] is None, "no fluorinated formula for unrelated ions"
            assert rows[4]["ev"] >= 5 and rows[4]["fr"] == ["CF3", "C2F5", "C3F7", "C4F9"], rows[4]
            assert 1 <= rows[2]["ev"] < rows[4]["ev"], rows[2]
            assert rows[7]["ev"] == 0 and rows[0]["ev"] is None, "unrelated ion: no evidence; ion never selected: no MS2"
            assert pg.locator("#pfas-plot g[data-i]").count() == 7 and pg.get_attribute("#pfas-plot", "role") == "img"
            assert pg.locator("#pfas-ev tbody tr").count() == 3 and "CF3" in pg.inner_text("#pfas-ev")
            t = pg.evaluate("PFAS.plotTable()"); assert len(t["rows"]) == 7 and t["head"][4] == "m/C", t["head"]
        step("PFAS: candidate formula, MD/C and m/C, fragments and losses of the MS2 as evidence; unrelated ions have none", pfas)
        def pfas_click():
            pg.locator("#pfas-plot g[data-i]").nth(4).dispatch_event("click")
            pg.wait_for_function("E.panels.some(p => p.pfasIon) && E.panels.some(p => p.pfasMs2)", timeout=30000)
            assert pg.locator("#pfas-tab tr.sel").count() == 1
        step("PFAS: a click opens the XIC and the MS2 scan", pfas_click)
        def unit():
            pg.select_option("#kmd-unit", "CH2"); pg.click("#kmd-go"); pg.wait_for_function("KMD.params.unit==='CH2'", timeout=10000)
            assert pg.evaluate("KMD.result.series.length") == 0 or pg.evaluate("KMD.result.series[0].members.length") < 7, "another unit does not regroup the CF2 series"
            pg.reload(); pg.wait_for_timeout(1500); assert pg.evaluate("KMD.params.unit") == "CH2", "the parameters come back"
        step("another unit regroups, the parameters are saved", unit)
finally:
    r.close(); r2.close()
r.report(); r2.report()
for s in steps: print(" ", s)
