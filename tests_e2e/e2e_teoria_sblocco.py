"""Administrator mode of the Teoria (sblocco.js): a single click on the logo still goes to the program; five quick clicks set qqq.admin, say it in a
role=status message and reload with html.esteso; with reduced motion nothing is animated; once unlocked the logo is a plain link again."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_sblocco.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
r = Run(port=8976, wd="/tmp/wd_teoria_sblocco")
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        def ctx(**kw):
            c = r.b.new_context(viewport={"width": 1280, "height": 900}, **kw); q = c.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e)))); return c, q
        ls = lambda q: q.evaluate("localStorage.getItem('qqq.admin')")
        def single():
            c, q = ctx(); q.goto(T + "01-tp.html"); q.wait_for_selector("#logo")
            assert q.get_attribute("#logo", "href") == "../../#/dati" and q.locator("#sblocco[role=status]").count() == 1
            q.click("#logo"); q.wait_for_url("**/#/dati", timeout=10000); assert ls(q) is None; c.close()
        step("one click still follows the logo link (after 600 ms) and unlocks nothing", single)
        def five():
            c, q = ctx(); q.goto(T + "01-tp.html"); q.wait_for_selector("#logo")
            assert not q.evaluate("document.documentElement.classList.contains('esteso')")
            for _ in range(5): q.click("#logo", delay=20, no_wait_after=True)
            q.wait_for_function("document.getElementById('sblocco').className==='on'", timeout=3000)
            assert q.inner_text("#sblocco").strip() == "Sbloccato" and ls(q) == "1"
            q.wait_for_function("document.documentElement.classList.contains('esteso')", timeout=8000)
            assert q.locator("#sblocco.on").count() == 0 and q.evaluate("TEORIA.esteso()") is True
            q.click("#logo"); q.wait_for_url("**/#/dati", timeout=3000)   # unlocked: a plain link, no delay, no counting
            c.close()
        step("five quick clicks: flag, status message, reload with html.esteso; then the logo is a plain link", five)
        def calm():
            c, q = ctx(reduced_motion="reduce"); q.goto(T + "01-tp.html"); q.wait_for_selector("#logo")
            for i in range(5):
                q.click("#logo", delay=20, no_wait_after=True)
                if i < 4: assert q.evaluate("document.querySelector('#logo img').getAnimations().length") == 0
            q.wait_for_function("localStorage.getItem('qqq.admin')==='1'", timeout=3000); c.close()
        step("reduced motion: no animation, same unlock", calm)
        def anim():
            c, q = ctx(); q.goto(T + "01-tp.html"); q.wait_for_selector("#logo")
            q.click("#logo", no_wait_after=True); assert q.evaluate("document.querySelector('#logo img').getAnimations().length") >= 1
            assert q.evaluate("getComputedStyle(document.querySelector('#logo')).display") != "none"; c.close()
        step("a click animates the logo image", anim)
        def editor():
            c, q = ctx(); q.goto(T + "01-tp.html"); q.wait_for_selector("#logo"); q.wait_for_selector("#a11y-btn", state="attached")
            q.wait_for_function("!!window.TEORIA_EDITOR", timeout=8000)          # editor.js has run: it only left the button out
            assert q.locator("#tc-editor").count() == 0
            q.keyboard.press("Alt+E")
            assert q.evaluate("!!document.querySelector('#tedit-bar')") is False and q.evaluate("document.designMode") != "on"
            for _ in range(5): q.click("#logo", delay=20, no_wait_after=True)
            q.wait_for_function("localStorage.getItem('qqq.admin')==='1'", timeout=3000)
            q.wait_for_function("document.documentElement.classList.contains('esteso')", timeout=8000)
            q.wait_for_selector("#tc-editor", timeout=5000)
            q.reload(); q.wait_for_selector("#tc-editor", timeout=8000)          # the state survives a reload
            q.click("#a11y-btn"); q.click("#a11y-admin-exit")                      # the way out, in the settings
            q.wait_for_function("localStorage.getItem('qqq.admin')===null", timeout=3000)
            q.wait_for_function("!document.documentElement.classList.contains('esteso')", timeout=8000)
            q.wait_for_function("!!window.TEORIA_EDITOR", timeout=8000); assert q.locator("#tc-editor").count() == 0; c.close()
        step("editor button and Alt+E only in administrator mode: off, 5 clicks, reload, exit", editor)
        def legacy():
            c, q = ctx(); q.add_init_script("try{localStorage.setItem('qqq.teoria.esteso','1')}catch(_){}"); q.goto(T + "01-tp.html")
            q.wait_for_selector("#tc-editor", timeout=8000); c.close()
        step("the older flag qqq.teoria.esteso still counts as administrator", legacy)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
