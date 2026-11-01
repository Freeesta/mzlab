# e2e_impostazioni_senza_dati: Settings has no «Data and privacy» group (no export/import of the user's data); «Report a bug» and the other groups stay.
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
r = Run(port=8880, wd="/tmp/wd_setpriv")
try:
    with sync_playwright() as p:
        for lang in ("it", "en"):
            pg = r.page(p); pg.goto(f"http://127.0.0.1:{r.port}/?lang={lang}"); attendi(pg, "!!document.querySelector('#np-set')", 30000)
            pg.click("#np-set"); pg.wait_for_selector("#uipset", state="visible")
            def gone(pg=pg):
                assert pg.locator("#uip-g-priv, #uip-export, #uip-import, #uip-import-file, #uip-export-st, #uip-import-box, #uip-import-st").count() == 0, "export/import controls still in Settings"
                assert "privacy" not in pg.inner_text("#uipset").lower(), pg.inner_text("#uipset")
            step(f"{lang}: no «Data and privacy» group", gone)
            def kept(pg=pg):
                ids = pg.evaluate("[...document.querySelectorAll('#uipset > details')].map(d => d.id)")
                for k in ("uip-g-look", "uip-a11y", "uip-g-bug"): assert k in ids, (k, ids)
            step(f"{lang}: the other groups (with «Report a bug») stay", kept)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
