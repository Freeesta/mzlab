"""Adducts tab of «Adducts & losses»: the standard adduct list shows by default (for the polarity in use), with no explanatory info line; typing filters, clearing brings back the last query's table; ESI+, ESI− or both; the two tabs only; the calculator folds. Reference lists name polymer series by repeat units, not «n=3»."""
import sys
from lib import *

if __name__ == "__main__":
    r = Run(port=8830 + 191, wd="/tmp/wd_addotti_default"); bad = []
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
            def step(n, f):
                try: f(); print("OK  ", n)
                except Exception as e: print("FAIL", n, repr(e)[:300]); bad.append(n)
            def default_table():
                pg.click("#stbar [data-cmd=addloss]"); wait_for_js(pg, "document.querySelector('#sb-panel-adducts .cadt tbody tr')")
                assert pg.input_value("#calcin") == ""
                t = pg.inner_text("#sb-panel-adducts")
                assert "[M+H]+" in t and "[M+Na]+" in t and "[M-H]-" not in t, t[:300]
                assert "A number is read" not in t and pg.locator("#sb-panel-adducts .nlhint").count() == 0
            step("default table in ESI+, no info line", default_table)
            def polarity():
                pg.click("#al-ctl [data-p='-1']"); wait_for_js(pg, "(t => t.includes('[M-H]-') && t.includes('[M+H]+'))(document.querySelector('#sb-panel-adducts .cadt').innerText)")      # both polarities
                pg.click("#al-ctl [data-p='1']"); wait_for_js(pg, "!document.querySelector('#sb-panel-adducts .cadt').innerText.includes('[M+Na]+')")
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
            def two_tabs():
                assert pg.locator("#al-tabs [role=tab]").count() == 2, "Adducts and Neutral losses only"
                assert pg.locator("#sb-panel-cont, #al-t-cont").count() == 0
                pg.click("#al-t-losses"); pg.fill("#calcin", "149.0233"); frames(pg, 30)
                assert pg.get_attribute("#al-t-losses", "aria-selected") == "true"
                pg.click("#al-t-adducts"); assert pg.input_value("#calcin") == "149.0233", "one field for the two tabs"
                pg.fill("#calcin", "")
            step("the page has exactly two tabs, one shared field", two_tabs)
            def old_saved_tab():
                # a state saved while a third tab existed falls back to Adducts
                n = pg.evaluate("""(() => { let n = 0; for (const k of Object.keys(localStorage)) { try { const o = JSON.parse(localStorage.getItem(k)); if (o && typeof o === 'object' && 'sub' in o) { o.sub = 'cont'; o.tab = 'addloss'; localStorage.setItem(k, JSON.stringify(o)); n++; } } catch (_) {} } return n; })()""")
                assert n >= 1, "the page saves its tab"
                pg.reload(); wait_for_js(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
                pg.evaluate("BARRA.setTab('addloss')"); wait_for_js(pg, "!!document.querySelector('#calcin').offsetParent")
                assert pg.get_attribute("#al-t-adducts", "aria-selected") == "true" and pg.evaluate("BARRA.curSub") == "adducts"
            step("a saved «contaminants» tab falls back to Adducts", old_saved_tab)
            def ripiega():
                assert pg.is_visible("#calcin") and pg.get_attribute("#al-calc-tg", "aria-expanded") == "true", "the calculator is shown by default"
                pg.click("#al-calc-tg"); wait_for_js(pg, "!document.querySelector('#calcpad').offsetParent")
                assert pg.get_attribute("#al-calc-tg", "aria-expanded") == "false"
                pg.reload(); wait_for_js(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
                pg.evaluate("BARRA.setTab('adducts')"); wait_for_js(pg, "!!document.querySelector('#calcin').offsetParent")
                assert not pg.is_visible("#calcpad") and pg.is_visible("#calcin"), "folded stays folded, the field stays"
                pg.evaluate("BARRA.setTab('calc')"); wait_for_js(pg, "!!document.querySelector('#calcpad').offsetParent")
                assert pg.get_attribute("#al-calc-tg", "aria-expanded") == "true", "the calc command unfolds it"
            step("the calculator folds with its toggle, remembered; the calc command unfolds it", ripiega)
            def ref_list():
                pg.evaluate("LISTE.open({})"); pg.wait_for_selector("#lst-tbody tr", timeout=8000)
                t = pg.evaluate("[...document.querySelectorAll('#lst-tbody tr')].map(r => r.innerText).join('\\n')")
                assert "n=" not in t, t[:300]
                assert pg.evaluate("LISTE.buildIndex([{id: 'k', name: 'k', items: [{id: 'peg', name: 'PEG (polyethylene glycol) n=3', mz: 150, pol: 1, series: {id: 'peg', n: 3}}]}], null).pos[0].name") == "PEG (polyethylene glycol), 3 repeat units"
            step("polymer series are not named «n=…» in the reference list", ref_list)
    finally:
        print("page errors:", r.errs); r.close()
    sys.exit(1 if bad else 0)
