"""Adducts & losses are ONE panel with one toolbar button and two tabs; the last tab used is remembered and survives a reload."""
import sys
from lib import *

if __name__ == "__main__":
    r = Run(port=8830 + 190, wd="/tmp/wd_addperd"); bad = []
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "window.qqReady && document.querySelector('#fl-hint')"); frames(pg, 5)
            def step(n, f):
                try: f(); print("OK  ", n)
                except Exception as e: print("FAIL", n, repr(e)[:300]); bad.append(n)
            def one_button():
                assert pg.locator("#stbar [data-cmd=addloss]").count() == 1
                assert pg.locator("#stbar [data-cmd=adducts], #stbar [data-cmd=losses]").count() == 0
            step("one toolbar button", one_button)
            def tabs():
                pg.click("#stbar [data-cmd=addloss]"); frames(pg, 6)
                assert pg.inner_text("#sb-title") == "Addotti e perdite"
                assert pg.get_attribute("#al-t-adducts", "aria-selected") == "true" and pg.is_visible("#sb-panel-adducts")
                pg.click("#al-t-losses"); frames(pg, 6)
                assert pg.is_visible("#sb-panel-losses") and not pg.is_visible("#sb-panel-adducts")
            step("two tabs switch the body", tabs)
            def remembered():
                pg.click("#stbar [data-cmd=addloss]"); frames(pg, 6)
                pg.click("#stbar [data-cmd=addloss]"); frames(pg, 6)
                assert pg.is_visible("#sb-panel-losses"), "the last tab is back"
                st = pg.evaluate("JSON.parse(localStorage.getItem('mzlab.barra.lr'))"); assert st["sub"] == "losses", st
            step("last tab remembered", remembered)
            def palette():
                pg.keyboard.press("Control+k"); attendi(pg, "document.querySelector('#pal-l')"); pg.keyboard.type("addotti")
                frames(pg, 6); assert "Addotti e perdite" in pg.inner_text("#pal-l"); pg.keyboard.press("Escape")
            step("palette finds the one command", palette)
    finally:
        print("page errors:", r.errs); r.close()
    sys.exit(1 if bad else 0)
