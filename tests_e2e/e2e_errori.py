"""One way to show errors: an uncaught error and an unhandled rejection give a short role=alert message with «Copy details»;
the workflow messages use the program dialog, never alert()."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8998, wd="/tmp/wd_errori")
steps = []

try:
    with sync_playwright() as p:
        pg = r.page(p)
        native = []
        pg.on("dialog", lambda d: (native.append(d.message), d.dismiss()))
        pg.wait_for_function("window.ERRORI")
        assert pg.locator("#errbox").count() == 0
        pg.evaluate("() => { setTimeout(() => { throw new Error('prova-errore-uno'); }, 0); }")
        pg.wait_for_selector("#errbox[role=alert]")
        txt = pg.inner_text("#errbox")
        assert "Qualcosa non ha funzionato" in txt and "Copia i dettagli" in txt, txt
        assert "prova-errore-uno" not in txt, "the message stays short: details only in the copy"
        steps.append(("error -> short alert box", "ok"))

        pg.evaluate("() => { window.__clip = ''; Object.defineProperty(navigator, 'clipboard', { value: { writeText: async t => { window.__clip = t; } }, configurable: true }); }")
        pg.evaluate("() => { Promise.reject(new Error('prova-rifiuto-due')); }")
        pg.wait_for_function("document.querySelector('#errbox .errmsg').textContent.includes('(2)')")
        pg.click("#errbox .errcopy")
        pg.wait_for_function("window.__clip")
        clip = pg.evaluate("window.__clip")
        assert "prova-errore-uno" in clip and "prova-rifiuto-due" in clip and "mzLab" in clip, clip
        steps.append(("copy details: version, route and stacks", "ok"))

        pg.click("#errbox .errx")
        assert pg.locator("#errbox").count() == 0
        steps.append(("close", "ok"))

        # workflow messages: program dialog, not alert()
        pg.evaluate("() => WORKFLOW.export()")
        pg.wait_for_selector("#askdlg[open]")
        assert "Nessun file aperto" in pg.inner_text("#asktxt")
        pg.click("#askok")
        pg.evaluate("() => WORKFLOW.applyWorkflow({formato: 'x'})")
        pg.wait_for_selector("#askdlg[open]")
        assert "non valido" in pg.inner_text("#asktxt")
        assert not native, native
        steps.append(("workflow messages in the program dialog", "ok"))

        print("OK: tutti i controlli superati")
        for desc, res in steps:
            print(f"  ✓ {desc}: {res}")
finally:
    r.close()
