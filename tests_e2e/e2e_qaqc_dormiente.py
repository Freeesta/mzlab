"""QA/QC dormant: with the switch off (default) nothing of it is in the DOM, the bar, «Altro», the palette, the tour or the notebook and the
console is quiet; with it on (?qaqc=1 / mzlab.dev.qaqc) the button, the panel and the palette command are there."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_qaqc_dormiente.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))

EX = ROOT / "mzlab" / "web" / "esempi" / "FullScan_t15.mzML"
HAS = "!!document.querySelector('#np-qaqc,#sb-panel-qaqc,[data-cmd=qaqc]')"

def palette(pg, q):
    pg.evaluate("document.activeElement.blur()"); pg.keyboard.press("Control+k"); pg.wait_for_selector("#paletta:not([hidden])")
    pg.keyboard.press("Control+a"); pg.keyboard.type(q); pg.wait_for_timeout(250)
    t = pg.inner_text("#pal-l"); pg.keyboard.press("Escape"); pg.wait_for_selector("#paletta", state="hidden"); return t

r = Run(port=8958, wd="/tmp/wd_qaqc_dorm")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1500, "height": 1000})
        stage(pg, [EX]); pg.click("text=Carica dati"); ready(pg)

        def spento():
            assert not pg.evaluate("window.FEATURES.qaqc") and not pg.evaluate(HAS), "no QA/QC element in the DOM"
            assert "QA/QC" not in pg.inner_text("#stbar"), pg.inner_text("#stbar")
            pg.click("#tb-more"); pg.wait_for_selector("#stmenu"); assert "QA/QC" not in pg.inner_text("#stmenu"); pg.keyboard.press("Escape")
        step("off (default): not in the DOM, the bar or «Altro»", spento)

        def paletta_spenta():
            for q in ("qaqc", "QA/QC", "controllo qualità", "rapporto"):
                assert "QA/QC" not in palette(pg, q), q
            assert "Nessun risultato" in palette(pg, "qaqc")
            assert "Rapporto di studio" in palette(pg, "rapporto"), "the study report stays"
        step("off: the palette neither finds nor shows it; the study report stays", paletta_spenta)

        def tour_taccuino():
            assert "qaqc" not in pg.evaluate("JSON.stringify(NB)").lower()
            for tab in ("file", "calc", "losses", "adducts", "lists", "stat"):
                drawer(pg, tab); assert "QA/QC" not in pg.inner_text("#sb-pane"), tab
            assert not pg.evaluate("(typeof TOUR!=='undefined'&&TOUR.steps?JSON.stringify(TOUR.steps):'')").count("qaqc")
            assert pg.evaluate("window.QAQC && typeof QAQC.roleOf") == "function", "roles (QC, dark) still work"
        step("off: not in the drawer, the tour or the notebook; the roles of the files remain", tour_taccuino)

        def pulita():
            bad = [e for e in r.errs if e[0] in ("pageerror", "console.error")]
            assert not bad, bad
        step("off: no errors in the console", pulita)
        r.close()

        r = Run(port=8959, wd="/tmp/wd_qaqc_dorm2"); pg = r.page(p, qaqc=True)
        pg.set_viewport_size({"width": 1500, "height": 1000})
        stage(pg, [EX]); pg.click("text=Carica dati"); ready(pg)

        def acceso():
            assert pg.evaluate("window.FEATURES.qaqc") and pg.locator("#stbar [data-cmd=qaqc]").count() == 1
            assert "QA/QC" in palette(pg, "qaqc")
            drawer(pg, "qaqc"); pg.wait_for_selector("#sb-panel-qaqc select[data-role]")
        step("on (mzlab.dev.qaqc): button, palette command and panel as before", acceso)
        r.close()
except Exception as e:
    steps.append(("setup", "FAIL " + str(e)[:400]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for n, s in steps) else 0)
