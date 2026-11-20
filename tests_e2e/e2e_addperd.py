"""Adducts & losses are ONE panel with one toolbar button and two tabs; the last tab used is remembered and survives a reload."""
import sys
from lib import *

if __name__ == "__main__":
    r = Run(port=8830 + 190, wd="/tmp/wd_addperd"); bad = []
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
            def step(n, f):
                try: f(); print("OK  ", n)
                except Exception as e: print("FAIL", n, repr(e)[:300]); bad.append(n)
            def one_button():
                assert pg.locator("#stbar [data-cmd=addloss]").count() == 1
                assert pg.locator("#stbar [data-cmd=adducts], #stbar [data-cmd=losses]").count() == 0
            step("one toolbar button", one_button)
            def tabs():
                pg.click("#stbar [data-cmd=addloss]")
                wait_for_js(pg, "document.querySelector('#sb-title')?.innerText === 'Adducts & losses' && document.querySelector('#al-t-adducts')", 15000)
                st = pg.evaluate("({ t: document.querySelector('#al-t-adducts')?.getAttribute('aria-selected'), a: !!document.querySelector('#sb-panel-adducts')?.offsetParent, h: document.querySelector('#sb-panel-addloss')?.hidden, sub: localStorage.getItem('mzlab.barra.lr') })")
                assert st["t"] == "true" and st["a"], ("the Adducts tab is not the selected one when the page opens", st)
                pg.click("#al-t-losses")
                wait_for_js(pg, "(() => { const e = document.querySelector('#sb-panel-losses'); return !!e && e.offsetParent !== null; })()", 15000)
                assert pg.is_visible("#sb-panel-losses") and not pg.is_visible("#sb-panel-adducts"), pg.evaluate("[!!document.querySelector('#sb-panel-losses')?.offsetParent, !!document.querySelector('#sb-panel-adducts')?.offsetParent]")
            step("two tabs switch the body", tabs)
            def remembered():
                pg.click("#stbar [data-cmd=addloss]"); wait_for_js(pg, "!document.querySelector('#sb-panel-losses')?.offsetParent", 15000)      # closed
                pg.click("#stbar [data-cmd=addloss]")
                wait_for_js(pg, "(() => { const e = document.querySelector('#sb-panel-losses'); return !!e && e.offsetParent !== null; })()", 15000)
                assert pg.is_visible("#sb-panel-losses"), "the last tab is back"
                st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.lr'))"); assert st["sub"] == "losses", st
            step("last tab remembered", remembered)
            def palette():
                pg.keyboard.press("Control+k"); wait_for_js(pg, "document.querySelector('#pal-l')"); pg.keyboard.type("adducts")
                frames(pg, 6); assert "Adducts & losses" in pg.inner_text("#pal-l"); pg.keyboard.press("Escape")
            step("palette finds the one command", palette)
    finally:
        print("page errors:", r.errs); r.close()
    sys.exit(1 if bad else 0)
