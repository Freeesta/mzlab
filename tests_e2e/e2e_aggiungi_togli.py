"""Adding and removing data in the single landing: «+» reopens the landing with «Back to work!» (Esc and the button give the work back as it was), the X of the toolbar
leaves the landing (a click on a file brings its panel back), removing every file shows the landing and Undo brings files and panels back, and a reload after
remove + Undo still shows the file (F8 of CARICAMENTO_STABILITA)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
FILES = [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_aggiungi_togli.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
WORK = "document.getElementById('v-workspace').dataset.work"
r = Run(port=8967, wd="/tmp/wd_aggiungi_togli")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.wait_for_selector("#drop"); attendi(pg, "window.qqReady === true", 60000)
        pg.set_input_files("#pick", FILES)
        attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === 2", 60000)
        pg.click("#flst .fl .nm >> nth=0"); pg.wait_for_selector(".pnl.chrom canvas", timeout=60000); ready(pg)
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        panels = lambda: pg.evaluate("E.panels.filter(p => p.el).length")
        def plus_and_back(leave):
            def go():
                n = panels(); pg.evaluate("document.querySelector('#addf').click()"); attendi(pg, f"{WORK} === 'add'")
                assert pg.is_visible("#drop") and pg.is_visible("#backwork"), "the landing with its drop area and «Back to work!»"
                assert pg.inner_text("#backwork").strip() in ("Back to work!", "Torna al lavoro!"), pg.inner_text("#backwork")
                assert pg.locator("#loaded, #cancadd, #backdata, #clearall").count() == 0, "no second landing"
                assert pg.locator("#start:visible").count() == 1
                if leave == "esc": pg.keyboard.press("Escape")
                else: pg.click("#backwork")
                attendi(pg, f"{WORK} === 'panels'"); assert panels() == n, "the panels are as they were"
                assert not pg.is_visible("#drop")
            return go
        step("«+» reopens the single landing; «Back to work!» returns", plus_and_back("button"))
        step("«+» then Esc returns", plus_and_back("esc"))
        def add_then_upload():
            pg.evaluate("document.querySelector('#addf').click()"); attendi(pg, f"{WORK} === 'add'")
            pg.set_input_files("#pick", [str(EX / "FullScan_t10.mzML")] if (EX / "FullScan_t10.mzML").exists() else FILES[:1])
            attendi(pg, "UPBUSY === 0 && OPENNOW.busy() === 0", 60000); attendi(pg, f"{WORK} === 'panels'")
            assert pg.evaluate("S.adding") is False
        step("a file added from the landing leaves it and the panels stay", add_then_upload)
        def x_landing():
            pg.click("#np-closeall"); attendi(pg, f"{WORK} === 'landing'")
            assert pg.is_visible("#drop") and pg.is_visible("#backwork") and pg.locator("#flst .fl").count() >= 2, "drop area, «Back to work!», files in the list"
            pg.click("#flst .fl .nm >> nth=0"); pg.wait_for_selector(".pnl canvas", timeout=30000); attendi(pg, f"{WORK} === 'panels'")
            assert not pg.is_visible("#drop")
            pg.click("#np-closeall"); attendi(pg, f"{WORK} === 'landing'"); pg.click("#backwork"); attendi(pg, f"{WORK} === 'panels'"); assert panels() >= 1
        step("closing every panel shows the landing; a file or «Back to work!» brings the panel back", x_landing)
        def remove_all():
            n = panels(); k = pg.evaluate("E.files.filter(f => !f.gone).length")
            pg.evaluate("removeAllFiles()"); attendi(pg, f"{WORK} === 'landing'")
            assert pg.is_visible("#drop") and pg.is_disabled("#np-chrom"), "landing, toolbar disabled"
            assert panels() == 0, "no empty panel is left"
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"E.files.filter(f => !f.gone).length === {k}")
            attendi(pg, f"{WORK} === 'panels'"); assert panels() == n, f"panels back: {panels()} of {n}"
        step("Remove all: landing, toolbar off, no empty panel; Undo brings files and panels back", remove_all)
        def f8():
            ks = pg.evaluate("E.files.filter(f => !f.gone).map(f => f.k)")
            pg.evaluate(f"E.files[{ks[-1]}].vis = false; E.files[{ks[0]}].vis = true; renderFileList()")
            pg.evaluate(f"removeFile(E.files[{ks[-1]}])"); pg.evaluate(f"removeFile(E.files[{ks[0]}])")
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"E.files[{ks[0]}].gone === false"); pg.wait_for_timeout(1500)
            pg.reload(); attendi(pg, "window.qqReady === true", 60000)
            attendi(pg, "E.files.some(f => f.vis && !f.gone)", 30000)
        step("reload after remove + Undo shows the file", f8)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
