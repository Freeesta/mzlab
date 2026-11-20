"""Interface language: English only, whatever the browser language is (it-IT, en-US, fr-FR); a language stored by an older version
(mzlab.lang) and ?lang= are ignored without a message. Synthetic data."""
# covers: mzlab/web/i18n.js, mzlab/web/lang/
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_lingua.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8830 + 100, wd="/tmp/wd_lingua")
try:
    with sync_playwright() as p:
        url = f"http://127.0.0.1:{r.port}/"
        b = p.chromium.launch()
        def page(locale, query="", stored=None):
            c = b.new_context(locale=locale, viewport={"width": 1500, "height": 1400})
            if stored: c.add_init_script("try{localStorage.setItem('mzlab.lang','%s')}catch(_){}" % stored)
            pg = c.new_page()
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(url + query); pg.wait_for_timeout(800); return pg
        nav = lambda pg: pg.evaluate("[...document.querySelectorAll('#nav button')].map(b => b.textContent.trim())")
        def english_everywhere():
            for loc in ("en-US", "it-IT", "fr-FR", "de-DE"):
                pg = page(loc); assert pg.evaluate("I18N.lang") == "en" and pg.evaluate("document.documentElement.lang") == "en", loc
                assert nav(pg) == ["Data", "Drawing", "Flow"], (loc, nav(pg))
        step("any browser language -> English (the browser language is not consulted)", english_everywhere)
        def query():
            pg = page("it-IT", "?lang=it"); assert pg.evaluate("I18N.lang") == "en" and nav(pg) == ["Data", "Drawing", "Flow"], nav(pg)
        step("?lang=it is ignored", query)
        def stored():
            pg = page("it-IT", stored="it"); assert pg.evaluate("I18N.lang") == "en" and nav(pg) == ["Data", "Drawing", "Flow"], nav(pg)
            pg.click("#np-set"); pg.wait_for_selector("#uipset")
            assert pg.locator("#uip-lang-l").count() == 0 and pg.locator("input[name=uip-lang]").count() == 0, "no language switch in the settings"
        step("a stored mzlab.lang=it is ignored silently, the settings have no language switch", stored)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
