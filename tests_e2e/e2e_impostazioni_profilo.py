"""Settings without a Profile section: the only switch is the administrator mode (5 clicks on the logo), which applies at once to an open panel; the old
explicit profile key is dropped; the section titles have more air above than below. No lab data needed.
Run from the repo root: python3 tests_e2e/e2e_impostazioni_profilo.py"""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8896, wd="/tmp/wd_impost_prof")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("localStorage.setItem('mzlab.profilo','ricerca'); localStorage.removeItem('mzlab.admin'); localStorage.removeItem('mzlab.teoria.esteso'); localStorage.setItem('mzlab.lang','en')")
        pg.reload(); ready(pg)
        def no_profile():
            assert pg.evaluate("localStorage.getItem('mzlab.profilo')") is None, "the old explicit profile is dropped"
            pg.click("#np-set"); pg.wait_for_selector("#uipset"); pg.evaluate("document.getElementById('uip-g-app').open = true")
            t = pg.inner_text("#uipset")
            for bad in ("Profile", "Profilo", "Course", "Corso", "Research", "Ricerca"): assert bad not in t, bad
            assert pg.locator("input[name=uip-prof]").count() == 0 and pg.locator("#uip-g-llm").count() == 0, "no assistant without the administrator mode"
        step("no Profile/Course/Research element; the old key is dropped; no assistant", no_profile)
        def five_clicks():
            pg.keyboard.press("Escape"); pg.evaluate("window.__noReload = 1")
            for _ in range(5): pg.click("header img.logo")
            pg.wait_for_function("localStorage.getItem('mzlab.admin') === '1'", timeout=10000)
            pg.click("#np-set"); pg.wait_for_selector("#uip-g-llm", timeout=10000)
            assert pg.evaluate("window.__noReload") == 1, "no reload"
            assert pg.locator("#uip-llm").count() == 1
        step("5 clicks on the logo: the hidden item appears in the settings, no reload", five_clicks)
        def titles():
            if not pg.locator("#uipset").count(): pg.click("#np-set"); pg.wait_for_selector("#uipset")
            pg.evaluate("document.querySelectorAll('#uipset details.imp-g').forEach(g => g.open = true)")
            for w in (1500, 380):
                pg.set_viewport_size({"width": w, "height": 900}); pg.wait_for_timeout(200)
                m = pg.evaluate("""[...document.querySelectorAll('#uipset .imp-b>fieldset')].map(f => [parseFloat(getComputedStyle(f).marginTop), parseFloat(getComputedStyle(f.querySelector(':scope>legend')).marginBottom), !!f.previousElementSibling])""")
                assert m, "sections found"
                for top, bot, has_prev in m:
                    if has_prev: assert top >= 18 and top >= 2 * bot, m
        step("section titles: more space above than below (1500 and 380 px)", titles)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
