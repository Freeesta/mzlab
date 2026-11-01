"""Adducts tab of «Addotti e perdite»: the standard adduct list shows by default (for the polarity in use), with no explanatory info line; typing filters, clearing brings back the last query's table. Reference lists name polymer series by repeat units, not «n=3»."""
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
                assert pg.input_value("#sb-panel-adducts [id$='-q']") == ""
                t = pg.inner_text("#sb-panel-adducts")
                assert "[M+H]+" in t and "[M+Na]+" in t and "[M-H]-" not in t, t[:300]
                assert "Un numero è letto" not in t and pg.locator("#sb-panel-adducts .nlhint").count() == 0
            step("default table in ESI+, no info line", default_table)
            def polarity():
                pg.click("#sb-panel-adducts [data-p='-1']"); attendi(pg, "document.querySelector('#sb-panel-adducts .cadt').innerText.includes('[M-H]-')")
                assert "[M+Na]+" not in pg.inner_text("#sb-panel-adducts .cadt")
                pg.click("#sb-panel-adducts [data-p='1']"); frames(pg, 20)
            step("the table follows ESI+/ESI-", polarity)
            def typing():
                pg.fill("#sb-panel-adducts [id$='-q']", "214"); frames(pg, 30)
                t = pg.inner_text("#sb-panel-adducts"); assert "[M+H]+" in t
                pg.fill("#sb-panel-adducts [id$='-q']", ""); frames(pg, 30)
                assert pg.locator("#sb-panel-adducts .cadt tbody tr").count() >= 4
            step("typing computes, clearing keeps a table", typing)
            def lista():
                pg.evaluate("LISTE.open({})"); pg.wait_for_selector("#lst-tbody tr", timeout=8000)
                t = pg.evaluate("[...document.querySelectorAll('#lst-tbody tr')].map(r => r.innerText).join('\\n')")
                assert "n=" not in t, t[:300]
                assert pg.evaluate("LISTE.buildIndex([{id: 'k', name: 'k', items: [{id: 'peg', name: 'PEG (polietilenglicole) n=3', mz: 150, pol: 1, series: {id: 'peg', n: 3}}]}], null).pos[0].name") == "PEG (polietilenglicole), 3 unità ripetitive"
            step("polymer series are not named «n=…» in the reference list", lista)
    finally:
        print("page errors:", r.errs); r.close()
    sys.exit(1 if bad else 0)
