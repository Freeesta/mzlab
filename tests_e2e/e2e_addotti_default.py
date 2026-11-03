"""Adducts tab of «Addotti e perdite»: the standard adduct list shows by default (for the polarity in use), with no explanatory info line; typing filters, clearing brings back the last query's table; ESI+, ESI− or both; the Contaminants tab; the calculator folds. Reference lists name polymer series by repeat units, not «n=3»."""
import sys
from lib import *

if __name__ == "__main__":
    r = Run(port=8830 + 191, wd="/tmp/wd_addotti_default"); bad = []
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
            def step(n, f):
                try: f(); print("OK  ", n)
                except Exception as e: print("FAIL", n, repr(e)[:300]); bad.append(n)
            def default_table():
                pg.click("#stbar [data-cmd=addloss]"); attendi(pg, "document.querySelector('#sb-panel-adducts .cadt tbody tr')")
                assert pg.input_value("#calcin") == ""
                t = pg.inner_text("#sb-panel-adducts")
                assert "[M+H]+" in t and "[M+Na]+" in t and "[M-H]-" not in t, t[:300]
                assert "Un numero è letto" not in t and pg.locator("#sb-panel-adducts .nlhint").count() == 0
            step("default table in ESI+, no info line", default_table)
            def polarity():
                pg.click("#al-ctl [data-p='-1']"); attendi(pg, "(t => t.includes('[M-H]-') && t.includes('[M+H]+'))(document.querySelector('#sb-panel-adducts .cadt').innerText)")      # both polarities
                pg.click("#al-ctl [data-p='1']"); attendi(pg, "!document.querySelector('#sb-panel-adducts .cadt').innerText.includes('[M+Na]+')")
                assert "[M-H]-" in pg.inner_text("#sb-panel-adducts .cadt")
                pg.click("#al-ctl [data-p='1']"); pg.click("#al-ctl [data-p='-1']"); frames(pg, 20)
                assert "[M-H]-" not in pg.inner_text("#sb-panel-adducts .cadt")
            step("the table follows ESI+, ESI- or both", polarity)
            def typing():
                pg.fill("#calcin", "214"); frames(pg, 30)
                t = pg.inner_text("#sb-panel-adducts"); assert "[M+H]+" in t
                pg.fill("#calcin", ""); frames(pg, 30)
                assert pg.locator("#sb-panel-adducts .cadt tbody tr").count() >= 4
            step("typing computes, clearing keeps a table", typing)
            def contaminanti():
                pg.click("#al-t-cont"); pg.fill("#calcin", "149.0233"); attendi(pg, "document.querySelectorAll('#sb-panel-cont li').length > 0")
                n = pg.locator("#sb-panel-cont li").count(); t = pg.inner_text("#sb-panel-cont")
                assert "149.0" in t and pg.get_attribute("#al-t-cont", "aria-selected") == "true", t[:300]
                assert pg.inner_text("#al-t-cont .cq-n").strip() == str(n) and pg.is_visible("#al-t-cont .cq-n"), "the tab counts its matches"
                pg.click("#al-t-adducts"); assert pg.input_value("#calcin") == "149.0233", "one field for the three tabs"
                pg.fill("#calcin", "C9H10N2O"); frames(pg, 30); assert not pg.is_visible("#al-t-cont .cq-n"), "a formula is not looked up in the contaminant lists"
                pg.fill("#calcin", "")
            step("Contaminants tab: the typed m/z in the known background lists, with its count", contaminanti)
            def ripiega():
                assert pg.is_visible("#calcin") and pg.get_attribute("#al-calc-tg", "aria-expanded") == "true", "the calculator is shown by default"
                pg.click("#al-calc-tg"); attendi(pg, "!document.querySelector('#calcin').offsetParent")
                assert pg.get_attribute("#al-calc-tg", "aria-expanded") == "false"
                pg.reload(); attendi(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
                pg.evaluate("BARRA.setTab('adducts')"); attendi(pg, "!!document.querySelector('#calcin').offsetParent")
                assert not pg.is_visible("#calcin"), "folded stays folded"
                pg.evaluate("BARRA.setTab('calc')"); attendi(pg, "!!document.querySelector('#calcin').offsetParent")
                assert pg.get_attribute("#al-calc-tg", "aria-expanded") == "true", "the calc command unfolds it"
            step("the calculator folds with its toggle, remembered; the calc command unfolds it", ripiega)
            def lista():
                pg.evaluate("LISTE.open({})"); pg.wait_for_selector("#lst-tbody tr", timeout=8000)
                t = pg.evaluate("[...document.querySelectorAll('#lst-tbody tr')].map(r => r.innerText).join('\\n')")
                assert "n=" not in t, t[:300]
                assert pg.evaluate("LISTE.buildIndex([{id: 'k', name: 'k', items: [{id: 'peg', name: 'PEG (polietilenglicole) n=3', mz: 150, pol: 1, series: {id: 'peg', n: 3}}]}], null).pos[0].name") == "PEG (polietilenglicole), 3 unità ripetitive"
            step("polymer series are not named «n=…» in the reference list", lista)
    finally:
        print("page errors:", r.errs); r.close()
    sys.exit(1 if bad else 0)
