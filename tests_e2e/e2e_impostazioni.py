"""Settings and tour: the tour invitation dismisses itself after a minute (counts as «Not now», never comes back), the Guided tour button of the settings
starts the tour in Dati also from Disegno, the storage section is gone (no reset: the header button is the only entry), the accessibility options sit in a submenu."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8873, wd="/tmp/wd_impost")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
try:
    with sync_playwright() as p:
        pg = r.page(p, tour=True)
        pg.evaluate("try { localStorage.removeItem('qqq.tour.lr'); localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1'); } catch (_) {}")
        pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); pg.wait_for_timeout(800)
        pg.click("text=Carica dati"); ready(pg)
        def invito():
            pg.wait_for_selector("#tour-invito", timeout=8000); pg.mouse.move(700, 100)
            t = pg.inner_text("#tour-invito"); assert "Primo accesso? Un tour guidato degli strumenti." in t and "minut" not in t, t
            hold(pg, 55000); assert pg.is_visible("#tour-invito"), "still there before a minute"
            hold(pg, 8000); assert not pg.is_visible("#tour-invito"), "dismissed by itself after a minute"
            assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.tour.lr')).stato") == "invitato"
            pg.reload(); ready(pg); pg.wait_for_timeout(1500); assert not pg.is_visible("#tour-invito"), "does not come back on reload"
        step("the invitation closes by itself after a minute and does not come back", invito)
        def gear():
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            t = pg.inner_text("#uipset")
            for bad in ("Archivio nel browser", "Storage in the browser", "Rimuovi file e indici", "Libera i derivati"): assert bad not in t, bad
            assert pg.locator("#uip-arch-all").count() == 0 and "Reset completo" not in t
            assert pg.locator("#uip-arch-rm").count() == 0 and pg.locator("#uip-arch-der").count() == 0
            assert pg.evaluate("document.getElementById('uip-a11y').open") is False
            assert pg.evaluate("[...document.querySelectorAll('#uip-a11y #uip-font, #uip-a11y #uip-hc, #uip-a11y #uip-pal')].length") == 3
            assert pg.locator("#uip-lib").count() == 0, "spectral libraries only with HRMS files"
            assert pg.evaluate("getComputedStyle(document.getElementById('uip-font')).appearance") == "none", "select without native arrow"
            pg.keyboard.press("Escape")
        step("settings: no storage section, one full reset, accessibility submenu closed", gear)
        def tour_from_draw():
            pg.click("#nav button[data-v=draw]"); pg.wait_for_timeout(800)
            assert pg.evaluate("S.view") == "draw"
            pg.click("#np-set"); pg.wait_for_selector("#uip-tour"); assert pg.is_enabled("#uip-tour")
            pg.click("#uip-tour")
            attendi(pg, "!!(window.TOUR && TOUR.active)")
            assert pg.evaluate("S.view") == "draw", "the tour of Disegno starts in Disegno"
        step("Guided tour from Disegno starts the tour of Disegno", tour_from_draw)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
