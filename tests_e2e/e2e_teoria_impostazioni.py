"""Settings of the Teoria (impostazioni.js): the gear opens ONE panel with Tema, Lettura facilitata and animations; none of the data settings; the theme
is the one of the program (qqq.prefs); «Ripristina predefinite» resets only the settings (never qqq.pratica / qqq.teoria.pos); choices persist."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_impostazioni.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
r = Run(port=8974, wd="/tmp/wd_teoria_impostazioni")
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        c = r.b.new_context(viewport={"width": 1280, "height": 900}); q = c.new_page()
        q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        q.goto(T + "13-formula.html"); q.wait_for_selector("#a11y-btn")
        ls = lambda k: q.evaluate(f"localStorage.getItem('{k}')")
        def content():
            q.click("#a11y-btn"); q.wait_for_selector("#a11y-panel:not([hidden])")
            legs = q.eval_on_selector_all("#a11y-panel legend", "e=>e.map(x=>x.textContent.trim())")
            for w in ("Tema", "Carattere", "Sfondo", "Lettura", "Animazioni"): assert w in legs, legs
            assert "Avanzamento" not in legs and q.locator("#a11y-ask,#a11y-forget-pos,#a11y-forget-pratica,[data-k=focus]").count() == 0
            txt = q.inner_text("#a11y-panel")
            for dead in ("Lingua", "Palette", "Etichette", "Unisci", "Librerie", "Alta risoluzione", "Aggiornamenti", "Tour"): assert dead not in txt, dead
            assert q.locator("#a11y-panel .hint").count() == 0, "no explanation lines under the controls"
            assert q.inner_text("#a11y-reset").strip() == "Ripristina predefinite"
        step("one panel: Tema, lettura, animazioni; no Avanzamento, no focus mode, no data settings", content)
        def theme():
            q.check("input[name=a11y-tema][value=dark]"); q.wait_for_function("document.documentElement.dataset.theme==='dark'", timeout=8000)
            assert json.loads(ls("qqq.prefs"))["theme"] == "dark"
            q.wait_for_selector("#a11y-btn"); q.click("#a11y-btn"); assert q.is_checked("input[name=a11y-tema][value=dark]")
            q.check("input[name=a11y-tema][value=auto]"); q.wait_for_timeout(500)
        step("Tema is the qqq.prefs of the program and survives a reload", theme)
        def motion():
            q.wait_for_selector("#a11y-btn"); q.click("#a11y-btn") if q.is_hidden("#a11y-panel") else None
            q.check("input[name=a11y-motion][value=always]"); assert q.get_attribute("html", "data-a11y-reduce") is not None
            assert json.loads(ls("qqq.a11y"))["motion"] == "always"
        step("reduced animations set data-a11y-reduce and persist", motion)
        def reset():
            q.evaluate("localStorage.setItem('qqq.pratica','{\"v\":1}');localStorage.setItem('qqq.teoria.pos','{\"last\":\"x\"}')")
            q.check("input[name=a11y-font][value=dyslexic]"); q.check("[data-k=space]"); assert q.get_attribute("html", "data-a11y-space") is not None
            q.click("#a11y-reset")
            assert q.get_attribute("html", "data-a11y-font") is None and q.get_attribute("html", "data-a11y-space") is None and q.get_attribute("html", "data-a11y-reduce") is None
            assert json.loads(ls("qqq.a11y")) == {}
            assert ls("qqq.pratica") is not None and ls("qqq.teoria.pos") is not None, "settings only"
            q.check("input[name=a11y-motion][value=always]")
        step("Ripristina predefinite: settings back to the defaults, progress and reading position kept", reset)
        def keys():
            q.evaluate("localStorage.setItem('qqq.pratica','{\"v\":1}')"); q.reload(); q.wait_for_selector("#a11y-btn")
            assert json.loads(ls("qqq.a11y"))["motion"] == "always" and ls("qqq.pratica")
        step("choices persist across a reload", keys)
        c.close()
        pg = r.page(p); pg.set_viewport_size({"width": 1280, "height": 900})
        def app():
            pg.click("#nav [data-v=theory]"); pg.wait_for_selector("#stbar [data-tcmd=indice]", timeout=15000); pg.click("#stbar [data-tcmd=impostazioni]")
            pg.wait_for_function("!document.getElementById('tframe').contentDocument.getElementById('a11y-panel').hidden", timeout=5000)
            assert pg.locator("#uipset").count() == 0
            pg.click("#nav [data-v=data]"); pg.click("#np-set"); pg.wait_for_selector("#uipset")
        step("in the program: gear opens the Teoria panel in Teoria, the data settings on Dati", app)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
