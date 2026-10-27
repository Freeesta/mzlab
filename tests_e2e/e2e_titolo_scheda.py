"""Browser tab title in administrator mode: every view (Dati, Disegno, Teoria, Flow, Tabui) sets its own document.title when the tab is switched, with no reload."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:1500]))
r = Run(port=8871, wd="/tmp/wd_titolo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.context.route("http://127.0.0.1:8765/**", lambda rt: rt.fulfill(status=200, content_type="application/json", headers={"access-control-allow-origin": "*"}, body='{"ok": false}'))
        for _ in range(5): pg.click("header img.logo")
        pg.wait_for_selector("#nav [data-v=flow]", timeout=30000); pg.wait_for_selector("#nav [data-v=tpmine]", timeout=30000)
        app = pg.evaluate("window.APP_NAME")
        def title(view, name):
            def f():
                pg.click(f"#nav [data-v={view}]")
                attendi(pg, f"document.title === {name!r}".replace("'", '"'), 10000)
            step(f"title of {view}", f)
        for view, name in [("theory", pg.evaluate("I18N.t('nav.theory')") + f" · {app}"), ("flow", f"Flow · {app}"), ("tpmine", f"Tabui · {app}"), ("draw", pg.evaluate("I18N.t('nav.draw')") + f" · {app}"), ("data", None)]:
            if name: title(view, name)
            else:
                def d():
                    pg.click("#nav [data-v=data]"); attendi(pg, 'document.querySelector("#v-data") && !document.querySelector("#v-data").hidden || !document.querySelector("#start").hidden', 10000); assert " · " not in pg.title(), pg.title()
                step("title of data", d)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
