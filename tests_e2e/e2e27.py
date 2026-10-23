"""XIC window (synthetic data): new text, one m/z field or neutral formula, unit window [n-0.2, n+0.8] around the nominal mass, one decimal, observed (clicked) centroid minus the drift, no '+ XIC' button."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e27.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8827 + 100, wd="/tmp/wd27")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        shutil.rmtree("/tmp/s27", ignore_errors=True)
        pg.set_input_files("#pick", synth.make_series("/tmp/s27", [0, 15, 60])); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        sm = lambda: pg.inner_text("#xic-sum")
        def dialog():
            pg.click("#np-xic"); t = pg.inner_text("#xicdlg") + " " + pg.get_attribute("#xicdlg", "aria-description")
            assert "Extracted Ion Chromatogram" in t and "1 Da" in t and "oppure" in t, t
            assert not pg.query_selector("#xic-lo") and "compromesso" not in t, t
            pg.fill("#xic-mz", "364.37"); assert "363.8 - 364.8" in sm(), sm()
            pg.fill("#xic-mz", "364,6"); assert "364.8 - 365.8" in sm(), sm()
            pg.fill("#xic-mz", "364.15"); assert "363.8 - 364.8" in sm()
            pg.fill("#xic-mz", "C14H13F4N3O2S"); pg.wait_for_timeout(1300)
            assert "363.8 - 364.8" in sm(), sm()
            pg.click("#xic-go"); ready(pg)
            tr = pg.evaluate("E.panels.find(p=>p.type==='xic').traces[0]"); assert tr["mz"] == 364.3 and tr["w"] == 0.5 and "m/z 364.3" in tr["label"], tr
        step("dialog: text, one value or formula, unit window, one decimal", dialog)
        def obs():
            pg.evaluate("openXic(null,{mz:364.37,obs:true})"); assert "363.8 - 364.8" in sm() and pg.input_value("#xic-mz") == "364.4", sm()
            pg.click("#xic-no")
            pg.evaluate("openXic(null,{mz:365.1,obs:true})"); assert "364.8 - 365.8" in sm(), sm(); pg.click("#xic-no")
        step("clicked centroid: drift taken off before the nominal mass (364.37 -> 364)", obs)
        def panel():
            assert pg.evaluate("document.querySelectorAll('.pnl.xic [data-o=addb]').length") == 0
            assert pg.input_value('.pnl.xic [data-o="xlo"]') == "363.8" and pg.input_value('.pnl.xic [data-o="xhi"]') == "364.8"
            pg.fill('.pnl.xic [data-o="xlo"]', "363.26"); pg.press('.pnl.xic [data-o="xlo"]', "Enter"); pg.wait_for_timeout(300); pg.locator('.pnl.xic [data-o="xhi"]').focus(); pg.wait_for_timeout(500)
            assert pg.input_value('.pnl.xic [data-o="xlo"]') == "363.3", pg.input_value('.pnl.xic [data-o="xlo"]')
        step("XIC panel: no + XIC button, fields with one decimal", panel)
        step("rh: half up on the decimal text", lambda: pg.evaluate("[rh(364.15),rh(0.25),rh(1.005,2),rh(2.675,2)]") == [364.2, 0.3, 1.01, 2.68] or (_ for _ in ()).throw(AssertionError(pg.evaluate("[rh(364.15),rh(0.25),rh(1.005,2),rh(2.675,2)]"))))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
