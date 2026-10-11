"""Method (.dam) upload: warning + button when missing, loading from the Metodo window; isotope bar spectrum; texts removed;
references for neutral losses."""
import sys, os, time; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:200]))
r = Run(port=8820, wd="/tmp/wd10")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def missing():
            pg.click("#np-method"); ready(pg)
            t = pg.inner_text("#bigdlg")
            assert "Manca il metodo di acquisizione" in t and "3200 QTRAP" in t, t[:200]
            assert "Parametri del metodo" not in t and "Curtain gas" not in t, "no built-in method may appear"
            assert pg.locator("#m-load").count() == 1
            pg.screenshot(path=SH + "100_no_method.png")
        step("no .dam: warning + load button, no invented parameters", missing)
        def load_here():
            with pg.expect_file_chooser() as fc: pg.click("#m-load")
            fc.value.set_files(str(DAM)); pg.wait_for_timeout(1500)
            t = pg.inner_text("#bigdlg")
            assert "Manca il metodo" not in t and "Curtain gas (CUR)" in t and "Metodo cromatografico (LC)" in t and "PDA" in t and DAM.name in t, t[:300]
            pg.screenshot(path=SH + "101_method_loaded.png")
        step("loading the .dam from the Metodo window shows all parameters", load_here)
        def two():
            with pg.expect_file_chooser() as fc: pg.click("#m-load")
            fc.value.set_files(str(DAM.parent / "Lab_inq_MRM_Flufe.dam")); pg.wait_for_timeout(1500)
            assert pg.locator("#m-sel option").count() == 2
            pg.select_option("#m-sel", index=0); pg.wait_for_timeout(500)
            assert "Curtain gas" in pg.inner_text("#bigdlg")
        step("two methods: selector", two)
        pg.click("#bigx"); pg.wait_for_timeout(300)
        def texts():
            pg.click("#np-ad"); pg.wait_for_timeout(400); pg.click("#stbar [data-cmd=losses]"); pg.wait_for_timeout(300)
            t = pg.inner_text("#sb-panel-losses"); assert "Differenze di massa frequenti" not in t and "Riferimenti" not in t and "Levsen" not in t
            assert pg.is_visible("#sb-help") and pg.locator("#sb-help").get_attribute("data-help") == "losses"
            pg.click("#np-pt"); pg.wait_for_timeout(300)             # the periodic table is in the header: it stays a window
            t = pg.inner_text("#refdlg"); assert "Passa sopra un elemento" not in t and "OpenChemLib" not in t and "CIAAW" not in t
            pg.screenshot(path=SH + "103_tables.png")
        step("removed texts; references listed", texts)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
