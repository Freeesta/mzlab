"""File list: «Full Scan» is always the first group (MS2 after it) whatever the load order; the Groups menu has «Experiment type» (checked by default) and «None» (a flat list) as separate items; no second × on the panel header of a wide screen."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
def same(a, b):
    assert a == b, a
r = Run(port=8899, wd="/tmp/wd_lista_ord")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / "MS2_t15.mzML")] + [str(ES / f"FullScan_t{t}.mzML") for t in (0, 15)])      # the MS2 file first
        pg.click("text=Carica dati"); ready(pg); drawer(pg)
        G = "[...document.querySelectorAll('#flst .fgh[data-gt]')].map(x => x.dataset.gt)"
        ITEMS = "[...document.querySelectorAll('.gby-menu [role=menuitemradio]')].slice(0, 2)"
        step("Full Scan group first, MS2 after", lambda: same(pg.evaluate(G)[:2], ["full", "ms2"]))
        def menu():
            pg.click("#sb-by"); pg.wait_for_selector(".gby-menu [role=menuitemradio]")
            same(pg.evaluate(ITEMS + ".map(x => [x.textContent, x.getAttribute('aria-checked')])"), [["Tipo di esperimento", "true"], ["Nessuno", "false"]])
            pg.click(".gby-menu [role=menuitemradio]:nth-child(2)"); ready(pg)
            assert pg.locator("#flst .fgh").count() == 0 and pg.locator("#flst .fl:not(.pr)").count() == 3, "flat list"
            pg.click("#sb-by"); pg.wait_for_selector(".gby-menu [role=menuitemradio]")
            same(pg.evaluate(ITEMS + ".map(x => x.getAttribute('aria-checked'))"), ["false", "true"])
            pg.click(".gby-menu [role=menuitemradio]:nth-child(1)"); ready(pg)
            same(pg.evaluate(G)[:2], ["full", "ms2"])
        step("Groups menu: Experiment type default, None = flat list, and back", menu)
        def english():
            pg.evaluate("I18N.set('en')"); pg.wait_for_function("I18N.lang === 'en'"); ready(pg)
            pg.click("#sb-by"); pg.wait_for_selector(".gby-menu [role=menuitemradio]")
            same(pg.evaluate(ITEMS + ".map(x => x.textContent)"), ["Experiment type", "None"])
            pg.keyboard.press("Escape")
        step("English menu", english)
        def nox():
            assert not pg.is_visible("#sb-x") and pg.is_visible("#sb-toggle")
            close_side(pg); attendi(pg, "(() => { const e = Q('#flst'); return !e || e.getBoundingClientRect().width === 0 })()")
        step("no second × on a wide screen; the header button closes the panel", nox)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
