"""Statistica, «Difference from the reference» (high resolution only): the section is there in HR and not in LR, the reference comes from the sample table,
sorting and filtering, the evidence of a row (XIC, MS1 at the apex, MS2, profile), the list column only with a list of the user, the export."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_differenza_hr.py")][-1]
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

# low resolution
LRD = HERE / "shots" / "difhr_lr_tmp"; LRD.mkdir(parents=True, exist_ok=True)
ds.full_scan(LRD / "S_t0.mzML", 0, np.random.default_rng(1)); ds.full_scan(LRD / "S_t30.mzML", 30, np.random.default_rng(2))

# high resolution: P appears with the time (absent at t0 and in the blank), Q (the parent) falls, B is a background ion that is everywhere
HRD = HERE / "shots" / "difhr_hr_tmp"; HRD.mkdir(parents=True, exist_ok=True)
P, Q, B = (305.1234, 6.0), (350.2000, 7.0), (410.3000, 8.0)
AMP = {"HR_t0": (0, 8e5), "HR_t10": (1e5, 6e5), "HR_t30": (4e5, 3e5), "HR_t60": (8e5, 1e5), "HR_Blank_1": (0, 0)}
for i, (name, (ap, aq)) in enumerate(AMP.items()):
    ds.hr_series(HRD / (name + ".mzML"), np.random.default_rng(10 + i), [(P[0], P[1], 0.06, ap), (Q[0], Q[1], 0.06, aq), (B[0], B[1], 0.06, 3e5)])
LIST = "305.1234; 6.0\n350.2000; 7.0\n410.3000; 8.0"

# high resolution with a data-dependent file: the click on a row opens the MS2 scan of the ion
DDD = HERE / "shots" / "difhr_dda_tmp"; DDD.mkdir(parents=True, exist_ok=True)
for i, n in enumerate(("D_t0", "D_t30", "D_Blank_1")):
    ds.hr_dda(DDD / (n + ".mzML"), np.random.default_rng(40 + i), "exploris")

r, r2, r3 = Run(port=8944, wd="/tmp/wddifhr"), Run(port=8945, wd="/tmp/wddifhr2"), Run(port=8946, wd="/tmp/wddifhr3")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [LRD / "S_t0.mzML", LRD / "S_t30.mzML"], 2)
        def lr_off():
            assert not pg.evaluate("BANCO.on()"), "low resolution"
            assert "reference" not in sections(pg), sections(pg)
        step("LR: the section does not exist", lr_off)

    with sync_playwright() as p:
        pg = r2.page(p)
        load(pg, [HRD / f"{n}.mzML" for n in AMP], len(AMP))
        def hr_on():
            assert pg.evaluate("BANCO.on()"), "high resolution"
            assert sections(pg) == ["meta", "profile", "features", "blank", "reference", "pca"], sections(pg)
            assert pg.is_visible("#stat-reference"), "the section is open"
            assert pg.locator("#ref-form").count() == 1
        step("HR: the section exists, after the blank subtraction", hr_on)
        def numbers():
            pg.fill("#feat-list", LIST); pg.click("#feat-go"); pg.wait_for_selector("#feat-tab tbody tr", timeout=60000)
            pg.wait_for_function("RIFERIMENTO.result && RIFERIMENTO.result.r.rows.length===3", timeout=30000)
            res = pg.evaluate("RIFERIMENTO.result.r"); names = pg.evaluate("RIFERIMENTO.result.names")
            assert sorted(names[i] for i in res["reference"]) == ["HR_Blank_1.mzML", "HR_t0.mzML"], ("the reference comes from the sample table", res["reference"], names)
            assert res["reference_rule"] == "metadata" and sorted(names[i] for i in res["treated"]) == ["HR_t10.mzML", "HR_t30.mzML", "HR_t60.mzML"]
            rows = res["rows"]
            assert rows[0]["kept"] and rows[0]["absent_in_reference"] and rows[0]["shape"] == "rises" and names[rows[0]["max_file"]] == "HR_t60.mzML", rows[0]
            assert not rows[1]["kept"] and rows[1]["ratio"] < 1 and rows[1]["shape"] == "falls" and rows[1]["reason"] == "below_reference", rows[1]
            assert not rows[2]["kept"] and rows[2]["shape"] is None and rows[2]["reason"] == "below_reference", rows[2]      # the background ion is in the blank: no label
            assert rows[0]["log2fc"] > 3 > rows[2]["log2fc"] > -1, [x["log2fc"] for x in rows]
        step("HR: P kept (absent in the reference, rises), the parent and the background discarded with the reason", numbers)
        def reference_in_clear():
            txt = pg.inner_text("#ref-used")
            assert "HR_t0.mzML" in txt and "HR_Blank_1.mzML" in txt and "HR_t60.mzML" in txt, txt
            assert pg.locator("input[data-ref]").count() == 5 and pg.locator("input[data-ref]:checked").count() == 2
            assert pg.evaluate("RIFERIMENTO.result.r.params.pseudo") > 0 and pg.inner_text("#ref-used").count("(") >= 2, "pseudocount and its origin are shown"
        step("HR: the reference, the treated files and the pseudocount are written in clear", reference_in_clear)
        def filters_and_sort():
            assert pg.locator("#ref-tab tbody tr").count() == 1, "kept only, by default"
            pg.select_option("#ref-show", "all"); assert pg.locator("#ref-tab tbody tr").count() == 3
            first = lambda: pg.evaluate("[...document.querySelectorAll('#ref-tab tbody tr')].map(r=>+r.dataset.mz)")
            assert first()[0] == 305.1234 and first()[-1] != 305.1234, ("log2FC descending", first())
            assert pg.get_attribute("#ref-tab th:nth-child(7)", "aria-sort") == "descending"
            pg.click("button[data-sort=log2fc]"); assert first()[0] != 305.1234 and pg.get_attribute("#ref-tab th:nth-child(7)", "aria-sort") == "ascending"
            pg.click("button[data-sort=shape]"); order = first()
            assert order[0] == 305.1234 and order[1] == 350.2, ("rises, falls, no label", order)
            pg.select_option("#ref-shape", "falls"); assert first() == [350.2]
            pg.select_option("#ref-shape", "new"); assert first() == [305.1234], "absent in the reference and kept"
            pg.select_option("#ref-shape", "all"); pg.select_option("#ref-show", "discarded"); assert pg.locator("#ref-tab tbody tr").count() == 2
            pg.select_option("#ref-show", "kept")
            assert pg.locator("#ref-tab thead th").count() == 16, "no list column without a list of the user"
            assert "TSV" in pg.inner_text("#ref-tsv") and pg.is_visible("#ref-xlsx")
        step("HR: sort by log2FC and by shape, filter by state and by shape, columns of mzmine", filters_and_sort)
        def keyboard():
            pg.focus("#ref-tab .ref-sort"); pg.keyboard.press("Enter")
            pg.wait_for_function("document.activeElement && document.activeElement.classList.contains('ref-sort')")
            assert pg.get_attribute("#ref-tab tbody tr", "role") == "button" and pg.get_attribute("#ref-tab tbody tr", "tabindex") == "0"
            assert "305.1234" in pg.get_attribute("#ref-tab tbody tr", "aria-label")
        step("HR: sorting and rows work with the keyboard, the rows have an aria-label", keyboard)
        def evidence():
            pg.focus("#ref-tab tbody tr"); pg.keyboard.press("Enter")
            pg.wait_for_function("E.panels.some(q=>q.type==='xic')", timeout=30000)
            pg.wait_for_function("E.panels.some(q=>q.type==='spec' && q.link!=null && q.level===1)", timeout=30000)
            assert abs(pg.evaluate("E.panels.filter(q=>q.type==='xic').pop().traces[0].mz") - 305.1234) < 0.01
            apex = pg.evaluate("E.files[E.panels.filter(q=>q.type==='spec').pop().k].file")
            assert apex == "HR_t60.mzML", ("the spectrum is the one of the file with the highest area", apex)
            pg.wait_for_function("PROFILO.result && PROFILO.result.rows.length>=5", timeout=30000)
            assert abs(float(pg.evaluate("PROFILO.state.text")) - 305.1234) < 1e-3
        step("HR: a row opens the XIC in the files, the MS1 at the apex of the highest file and the profile in time", evidence)
        def user_list():
            assert pg.evaluate("LISTE.hasUser()") is False
            pg.evaluate("LISTE.addUserListFromCsv('mine.csv','name,mz\\nTest-A,305.1235\\nTest-B,350.9\\n','Mine')")
            pg.evaluate("document.dispatchEvent(new CustomEvent('campioni'))")
            pg.wait_for_function("document.querySelectorAll('#ref-tab thead th').length===17", timeout=30000)
            txt = pg.inner_text("#ref-tab tbody tr")
            assert "Test-A" in txt and "ppm" in txt and "Test-B" not in txt, txt
        step("HR: the column «in your list» appears only with a list of the user, with name and Δppm", user_list)
        def choose_reference():
            drawer(pg, "stat")
            pg.uncheck("input[data-ref='HR_t0.mzML']")
            pg.wait_for_function("RIFERIMENTO.result && RIFERIMENTO.result.r.reference_rule==='explicit'", timeout=30000)
            res = pg.evaluate("RIFERIMENTO.result"); names = res["names"]
            assert [names[i] for i in res["r"]["reference"]] == ["HR_Blank_1.mzML"]
            assert res["rows"][1]["q"]["kept"], "with only the blank as the reference the parent is kept: the choice is the user's"
            assert "HR_t0.mzML" in pg.inner_text("#ref-used") and pg.locator("input[data-ref]:checked").count() == 1
            pg.click("#ref-default")
            pg.wait_for_function("RIFERIMENTO.result && RIFERIMENTO.result.r.reference_rule==='metadata'", timeout=30000)
            assert pg.locator("input[data-ref]:checked").count() == 2 and not pg.evaluate("RIFERIMENTO.result.rows[1].q.kept")
        step("HR: the user chooses the reference; the default is one click away", choose_reference)
        def export():
            with pg.expect_download() as dl_:
                pg.click("#ref-tsv")
            txt = Path(dl_.value.path()).read_text(encoding="utf-8")
            lines = txt.splitlines()
            assert any(l.startswith("# ") and "HR_t0.mzML" in l and "HR_Blank_1.mzML" in l for l in lines), "the reference used is written"
            assert any("pseudo" in l.lower() for l in lines if l.startswith("# ")) and any(l.startswith("row ID\trow m/z\trow retention time") for l in lines), lines[:20]
            assert len([l for l in lines if not l.startswith("#")]) == 1 + 1, "header and the kept row"
            with pg.expect_download() as dl2:
                pg.click("#ref-xlsx")
            assert dl2.value.suggested_filename.endswith(".xlsx")
        step("HR: TSV and Excel with the parameters and the reference used", export)
        def errors_clear():
            res = pg.evaluate("""async () => { const r = await fetch('api/stat/reference', {method:'POST', headers:{'content-type':'application/json'},
                body: JSON.stringify({matrix:[[1,2]], roles:['sample','sample'], times:[5,10]})}); return await r.json(); }""")
            assert res["error_key"] == "err.stat.ref.none", res
        step("HR: no blank and no file at time 0 gives a clear message", errors_clear)

    with sync_playwright() as p:
        pg = r3.page(p)
        load(pg, [DDD / f"{n}.mzML" for n in ("D_t0", "D_t30", "D_Blank_1")], 3)
        def ms2():
            sections(pg)
            pg.fill("#feat-list", "305.0702; 9.0"); pg.click("#feat-go"); pg.wait_for_selector("#feat-tab tbody tr", timeout=120000)
            pg.wait_for_function("RIFERIMENTO.result", timeout=30000)
            pg.select_option("#ref-show", "all")
            pg.click("#ref-tab tbody tr")
            pg.wait_for_function("E.panels.some(q=>q.dda!=null)", timeout=60000)
            pg.wait_for_function("E.panels.some(q=>q.dda!=null && q.sid!=null)", timeout=30000)
        step("HR with a DDA file: a row opens the MS2 scan of the ion", ms2)
finally:
    for rr in (r, r2, r3): rr.close()
for s_ in steps: print(s_)
errs = [e for rr in (r, r2, r3) for e in rr.errs if e[0] != "console.warning"]
print("ERRORS", errs[:5])
sys.exit(1 if [s_ for s_ in steps if s_[1] != "ok"] or errs else 0)
