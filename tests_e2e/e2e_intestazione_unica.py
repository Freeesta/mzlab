"""One header for the whole program: the standalone Teoria (direct load or reload) has the same compact height and the same tab language as the header of the program,
with admin off and on, at 1440 and 400 px; full screen button in both; no floating «Aa» in the Teoria of the program; the «only in Italian» notice shows once."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8981, wd="/tmp/wd_iu")
HEIGHT = "(() => Math.round((document.querySelector('#top') || document.querySelector('header')).getBoundingClientRect().height))()"
TABS = "[...document.querySelectorAll('#tabs a, #nav button')].map(e => e.textContent.trim())"
try:
    with sync_playwright() as p:
        r.page(p)
        U = f"http://127.0.0.1:{r.port}/"; T = U + "static/teoria/12-dati.html"
        def new(w, lang, admin):
            c = r.b.new_context(viewport={"width": w, "height": 800}); q = c.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            adm = 'localStorage.setItem("mzlab.admin", "1")' if admin else 'localStorage.removeItem("mzlab.admin")'
            q.add_init_script("try { localStorage.setItem('mzlab.lang', '" + lang + "'); " + adm + " } catch (e) {}")
            return c, q
        def same():
            for lang, names in (("it", ["Dati", "Disegno", "Teoria"]), ("en", ["Data", "Drawing", "Theory"])):
                hs = {}
                for adm in (0, 1):
                    c, q = new(1440, lang, adm); q.goto(T); q.wait_for_selector("#tbar #tc-schermo")
                    assert q.evaluate(TABS) == names, (lang, q.evaluate(TABS)); hs[adm] = q.evaluate(HEIGHT); c.close()
                assert abs(hs[0] - hs[1]) <= 2, ("admin on/off", hs)
                c, q = new(1440, lang, 0); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
                assert q.evaluate(TABS) == names, (lang, q.evaluate(TABS)); app = q.evaluate(HEIGHT); c.close()
                assert abs(app - hs[0]) <= 2, (lang, "program", app, "Teoria", hs)
        step("1440 px: the Teoria header has the height and the tab language of the program, admin off and on", same)
        def narrow():
            hs = []
            for adm in (0, 1):
                c, q = new(400, "en", adm); q.goto(T); q.wait_for_selector("#tabs")
                hs.append(q.evaluate(HEIGHT)); assert q.evaluate("document.documentElement.scrollWidth") <= 400; c.close()
            assert abs(hs[0] - hs[1]) <= 2 and hs[0] <= 60, hs
        step("400 px: compact, the same with admin off and on, no sideways scroll", narrow)
        def fullscreen():
            c, q = new(1440, "en", 0); q.goto(T); q.wait_for_selector("#tc-schermo")
            assert q.get_attribute("#tc-schermo", "aria-label") == "Full screen" and q.get_attribute("#tc-schermo", "aria-pressed") == "false"
            b = q.locator("#tc-schermo").bounding_box(); assert b["width"] >= 24 and b["height"] >= 24, b
            q.click("#tc-schermo"); attendi(q, "!!document.fullscreenElement", 5000) if q.evaluate("!!document.documentElement.requestFullscreen") else None
            c.close()
        step("standalone Teoria: full screen button with its name and a real target", fullscreen)
        def noaa():
            c, q = new(1440, "it", 0); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
            q.click("#nav [data-v=theory]"); attendi(q, "(() => { const f = document.querySelector('#tframe'); try { return !!f.contentDocument.querySelector('#a11y-panel') } catch (e) { return false } })()", 60000)
            f = next(x for x in q.frames if "/teoria/" in x.url)
            assert f.evaluate("!document.querySelector('#a11y-btn')"), "no Aa in the Teoria of the program"
            q.locator("header").get_by_role("button", name="Impostazioni").click(); f.wait_for_selector("#a11y-panel:not([hidden])"); c.close()
        step("Teoria in the program: no «Aa», the gear of the header opens the settings", noaa)
        def bug():
            for adm in (0, 1):
                c, q = new(1440, "it", adm); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
                assert q.is_visible("#bug-btn"), ("bug button", adm); c.close()
        step("the bug button shows for everybody, admin off and on", bug)
        def once():
            c, q = new(1440, "en", 0); q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000)
            sorry = "[...document.querySelectorAll('#toast-container *')].filter(e => e.textContent.trim() === 'Sorry! For now the theory is only in Italian.').length"
            q.click("#nav [data-v=theory]"); attendi(q, sorry + " > 0", 10000)
            attendi(q, sorry + " === 0", 10000)                                       # it goes away by itself
            q.click("#nav [data-v=data]"); q.click("#nav [data-v=theory]"); frames(q, 30)
            assert q.evaluate(sorry) == 0, "second entry: no notice"
            q.goto(U); q.wait_for_function("window.qqReady===true", timeout=60000); q.click("#nav [data-v=theory]"); frames(q, 30)
            assert q.evaluate(sorry) == 0, "after reload: no notice"; c.close()
        step("«only in Italian» notice: first entry only, not again after switching tab or reloading", once)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
