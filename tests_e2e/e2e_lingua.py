"""Interface language: default from the browser (it-IT -> Italian, en-US/fr-FR -> English), ?lang=, choice from the gear (remembered in
qqq.lang, files stay open after the reload), the Teoria in English is a courtesy page with a button to the Italian one. Synthetic data."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import synth
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
        def page(locale, query=""):
            c = b.new_context(locale=locale, viewport={"width": 1500, "height": 1400}); pg = c.new_page()
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(url + query); pg.wait_for_timeout(800); return pg
        nav = lambda pg: pg.evaluate("[...document.querySelectorAll('#nav button')].map(b => b.textContent)")
        def italian():
            pg = page("it-IT"); assert pg.evaluate("I18N.lang") == "it" and pg.evaluate("document.documentElement.lang") == "it"
            assert nav(pg) == ["Dati", "Disegno", "Teoria"], nav(pg)
        step("it-IT browser -> Italian", italian)
        def english():
            for loc in ("en-US", "fr-FR", "de-DE"):
                pg = page(loc); assert pg.evaluate("I18N.lang") == "en" and pg.evaluate("document.documentElement.lang") == "en", loc
                assert nav(pg) == ["Data", "Drawing", "Theory"], (loc, nav(pg))
        step("en-US, fr-FR, de-DE browsers -> English", english)
        def query():
            pg = page("it-IT", "?lang=en"); assert pg.evaluate("I18N.lang") == "en"
            pg = page("en-US", "?lang=it"); assert pg.evaluate("I18N.lang") == "it"
        step("?lang= forces the language for the tab", query)
        def theory():
            pg = page("en-US"); pg.click("#nav [data-v=theory]"); pg.wait_for_timeout(300)
            assert pg.is_visible("#theory-sorry") and not pg.is_visible("#tframe")
            t = pg.inner_text("#theory-sorry")
            assert "Sorry — the Theory section is not available in English yet. An English version is planned for a future release." in t, t
            assert pg.inner_text("#theory-it") == "Open the Italian version anyway"
            pg.click("#theory-it"); pg.wait_for_timeout(500)
            assert pg.is_visible("#tframe") and not pg.is_visible("#theory-sorry") and "teoria" in pg.evaluate("document.querySelector('#tframe').src")
            pg.click("#nav [data-v=data]"); pg.click("#nav [data-v=theory]"); pg.wait_for_timeout(300)
            assert pg.is_visible("#theory-sorry"), "the Italian Theory is only for that visit"
        step("Theory in English: courtesy page, button opens the Italian one for that visit only", theory)
        def italian_theory():
            pg = page("it-IT"); pg.click("#nav [data-v=theory]"); pg.wait_for_timeout(500)
            assert pg.is_visible("#tframe") and not pg.is_visible("#theory-sorry")
        step("Theory in Italian: unchanged", italian_theory)
        def gear():
            pg = page("it-IT"); shutil.rmtree("/tmp/s_lingua", ignore_errors=True)
            stage(pg, synth.make_series("/tmp/s_lingua", [0, 15, 60])); pg.click("text=Carica dati"); ready(pg)
            n = pg.evaluate("E.files.length"); assert n >= 3
            pg.click("#np-set"); pg.wait_for_timeout(200)
            assert pg.inner_text("#uipset label, #uipset .row >> nth=0").startswith("Lingua / Language")
            assert pg.evaluate("document.querySelector('#uip-lang').value") == "it"
            saved(pg)        # the open files come back from the saved session: wait until it is written
            with pg.expect_navigation(): pg.select_option("#uip-lang", "en")
            attendi(pg, "typeof I18N !== 'undefined' && I18N.lang === 'en'", 60000); ready(pg)
            assert pg.evaluate("I18N.lang") == "en" and pg.evaluate("localStorage.getItem('qqq.lang')") == "en"
            assert nav(pg) == ["Data", "Drawing", "Theory"]
            attendi(pg, f"E.files.length === {n}", 60000)
            assert pg.evaluate("E.files.length") == n, "open files must come back after the language change"
            pg.click("#np-set"); pg.wait_for_timeout(200); assert pg.evaluate("document.querySelector('#uip-lang').value") == "en"
            assert pg.inner_text("#uipset .row >> nth=0").startswith("Lingua / Language"), "the label stays bilingual"
        step("gear: choice remembered (qqq.lang), page reloads in English, open files stay", gear)
        def remembered():
            pg = page("it-IT"); pg.evaluate("localStorage.setItem('qqq.lang','en')"); pg.reload(); pg.wait_for_timeout(600)
            assert pg.evaluate("I18N.lang") == "en", "the explicit choice wins over the browser"
        step("explicit choice wins over the browser language", remembered)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
