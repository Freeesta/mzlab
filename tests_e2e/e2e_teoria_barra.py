"""Commands of the Teoria header (comandi.js, TCMD): order and names, previous/next, search, glossary, formulas, exercises, tools menu (keyboard),
gear, labels that give way when the row is narrow, narrow screens (previous/next in the fixed bar), phone («Altro»), targets, axe."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_barra.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
def axe_path():
    p = os.environ.get("AXE_JS")
    if p and os.path.exists(p): return p
    try:
        import axe_playwright_python
        q = os.path.join(os.path.dirname(axe_playwright_python.__file__), "axe.min.js")
        return q if os.path.exists(q) else None
    except ImportError: return None
AXE = axe_path()
r = Run(port=8971, wd="/tmp/wd_teoria_barra")
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        def new(w=1280, h=900, ctx=False, **kw):
            c = r.b.new_context(viewport={"width": w, "height": h}, reduced_motion="reduce", **kw); q = c.new_page()
            q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            return q
        def page(q, f):
            q.goto(T + f); q.wait_for_selector("#tbar"); q.wait_for_selector("#a11y-btn"); q.wait_for_function("typeof TEORIA!=='undefined' && typeof TEORIA.cerca==='function' && typeof TEORIA.strumenti==='object'")
        q = new(); page(q, "13-formula.html")
        def order():
            ids = q.eval_on_selector_all("#tbar > *, #tbar .tgrp > *", "e=>e.filter(x=>x.matches('a,button')).map(x=>x.id)")
            assert ids == ["tc-prec", "tc-succ", "tc-cerca", "tc-glossario", "tc-formule", "tc-esercizi", "tc-strumenti", "a11y-btn"], ids
            names = q.eval_on_selector_all("#tbar a,#tbar button", "e=>e.map(x=>x.getAttribute('aria-label'))")
            assert names[2].startswith("Cerca") and "Glossario" in names and "Formule" in names and "Esercizi" in names and names[-1].startswith("Impostazioni"), names
            for sel in ("#tc-cerca", "#tc-glossario", "#tc-formule", "#tc-esercizi", "#tc-strumenti"):
                lab = q.inner_text(sel).strip().rstrip("▾").strip(); assert lab and lab in q.get_attribute(sel, "aria-label"), (sel, lab)   # visible label in the name (WCAG 2.5.3)
            for t in ("Dati", "Disegno", "Teoria"): assert q.is_visible(f"#tabs >> text={t}")
            for dead in ("Calculator", "Losses", "Adducts", "Lists", "Table", "More", "Search", "New session", "Segnalibri", "Nuova sessione"): assert not q.query_selector(f"#tbar >> text='{dead}'"), dead
        step("order, names, labels in the names, tabs kept, no data or discarded commands", order)
        def targets():
            for s in ("#tc-prec", "#tc-succ", "#tc-cerca", "#tc-glossario", "#tc-formule", "#tc-esercizi", "#tc-strumenti", "#a11y-btn"):
                b = q.locator(s).bounding_box(); assert b["height"] >= 44 and b["width"] >= 44, (s, b)
        step("targets at least 44x44 px", targets)
        def prevnext():
            n = q.get_attribute("#tc-succ", "href"); assert n == "14-frammentazione-esi.html", n
            q.click("#tc-succ"); q.wait_for_url("**/14-frammentazione-esi.html")
            q.wait_for_selector("#tbar"); assert q.get_attribute("#tc-prec", "href") == "13-formula.html"
            q.click("#tc-prec"); q.wait_for_url("**/13-formula.html"); q.wait_for_selector("#tbar")
            q.goto(T + "00-uso.html"); q.wait_for_selector("#tbar"); assert q.get_attribute("#tc-prec", "aria-disabled") == "true" and q.get_attribute("#tc-prec", "href") is None
            q.goto(T + "13-formula.html"); q.wait_for_selector("#tbar")
        step("previous / next change the chapter, the first has no previous", prevnext)
        def cerca():
            q.click("#tc-cerca"); q.wait_for_selector("#cerca-in", state="visible"); assert q.evaluate("TEORIA.cerca.aperta()")
            q.keyboard.press("Escape"); q.wait_for_selector("#cerca-bg", state="hidden")
            assert q.evaluate("document.activeElement.id") == "tc-cerca"
            q.keyboard.press("Control+k"); q.wait_for_selector("#cerca-in", state="visible"); q.keyboard.press("Escape")
        step("Cerca opens the dialog (button and Ctrl+K), Esc gives the focus back", cerca)
        def links():
            q.click("#tc-glossario"); q.wait_for_url("**/21-glossario.html"); q.wait_for_selector("#tbar")
            q.click("#tc-formule"); q.wait_for_url("**/22-formulario.html"); q.wait_for_selector("#tbar"); q.goto(T + "13-formula.html"); q.wait_for_selector("#tbar")
        step("Glossario and Formule go to the chapters 21 and 22", links)
        def esercizi():
            q.goto(T + "16-ei-famiglie.html"); q.wait_for_selector("#tbar"); q.click("#tc-esercizi"); q.wait_for_timeout(300)
            assert q.evaluate("document.activeElement.classList.contains('gym')"), q.evaluate("document.activeElement.outerHTML.slice(0,80)")
            q.goto(T + "04-lc.html"); q.wait_for_selector("#tbar"); q.click("#tc-esercizi"); q.wait_for_timeout(300)
            assert q.evaluate("/^Verifica/.test(document.activeElement.textContent.trim())")
            q.goto(T + "pratica-ei.html"); q.wait_for_selector("#tbar"); q.click("#tc-esercizi")
        step("Esercizi goes to the «Allenati» box, else «Verifica»", esercizi)
        def strum():
            q.goto(T + "14-frammentazione-esi.html"); q.wait_for_selector("#tbar"); q.wait_for_function("typeof TEORIA.strumenti==='object'")
            assert q.get_attribute("#tc-strumenti", "aria-haspopup") == "menu" and q.get_attribute("#tc-strumenti", "aria-expanded") == "false"
            q.focus("#tc-strumenti"); q.keyboard.press("Enter"); q.wait_for_selector(".tmenu", state="visible")
            assert q.get_attribute("#tc-strumenti", "aria-expanded") == "true"
            items = q.eval_on_selector_all(".tmenu [role=menuitem]", "e=>e.map(x=>x.textContent)"); assert items == ["Frammenta", "Massa e addotti", "Tavola periodica", "Perdite"], items
            assert q.evaluate("document.activeElement.textContent") == "Frammenta"
            q.keyboard.press("ArrowDown"); assert q.evaluate("document.activeElement.textContent") == "Massa e addotti"
            q.keyboard.press("Escape"); assert q.is_hidden(".tmenu") and q.evaluate("document.activeElement.id") == "tc-strumenti"
            q.keyboard.press("ArrowDown"); q.keyboard.press("ArrowDown"); q.keyboard.press("ArrowDown"); q.keyboard.press("Enter")
            q.wait_for_function("TEORIA.strumenti.aperto()==='tavola'"); assert q.is_hidden(".tmenu")
            q.keyboard.press("Escape"); q.wait_for_function("TEORIA.strumenti.aperto()===''")
            q.click("#tc-strumenti"); q.click(".tmenu >> text=Massa e addotti"); q.wait_for_function("TEORIA.strumenti.aperto()==='massa'")
            q.evaluate("TEORIA.strumenti.chiudi()")
            q.click("#tc-strumenti"); q.click(".tmenu >> text=Frammenta"); q.wait_for_function("TEORIA.strumenti.aperto()==='frammenta'"); q.evaluate("TEORIA.strumenti.chiudi()")
        step("Strumenti menu: keys, Esc, the four entries open the tools", strum)
        def gear():
            q.click("#a11y-btn"); assert q.is_visible("#a11y-panel"); q.keyboard.press("Escape"); assert q.is_hidden("#a11y-panel")
        step("the gear opens the reading settings", gear)
        def fit():
            q.set_viewport_size({"width": 1000, "height": 800}); q.wait_for_timeout(300)
            assert q.evaluate("document.querySelector('#top').scrollWidth<=document.querySelector('#top').clientWidth")
            q.set_viewport_size({"width": 1280, "height": 900}); q.wait_for_timeout(300)
            assert q.is_visible("#tc-cerca .l")
        step("labels give way when the row is narrow, no overflow", fit)
        q.close()
        def narrow():
            d = new(820, 900); page(d, "13-formula.html")
            assert d.is_visible("#cb-succ") and d.is_visible("#cb-prec") and d.is_visible("#menu")
            assert d.is_visible("#tc-cerca") and d.is_visible("#tc-strumenti") and d.is_visible("#a11y-btn")
            assert not d.is_visible("#tc-glossario") and not d.is_visible("#tc-prec")
            for s in ("#cb-prec", "#cb-succ", "#menu", "#tc-cerca", "#tc-strumenti", "#a11y-btn"):
                b = d.locator(s).bounding_box(); assert b["height"] >= 44 and b["width"] >= 44, (s, b)
            assert d.evaluate("document.documentElement.scrollWidth") <= 820
            d.click("#cb-succ"); d.wait_for_url("**/14-frammentazione-esi.html"); d.close()
        step("<= 980 px: previous/next in the fixed bar, search, tools and gear at the top", narrow)
        def phone():
            c = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, reduced_motion="reduce"); d = c.new_page()
            d.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            page(d, "06-esi.html")
            assert d.is_visible("#tc-cerca") and d.is_visible("#a11y-btn") and d.is_visible("#info") and d.is_visible("#tc-altro")
            assert not d.is_visible("#tc-strumenti") and not d.is_visible("#tc-glossario")
            for s in ("#tc-cerca", "#a11y-btn", "#info", "#tc-altro"):
                b = d.locator(s).bounding_box(); assert b["height"] >= 40 and b["width"] >= 40, (s, b)
            assert d.evaluate("document.documentElement.scrollWidth") <= 390
            d.tap("#tc-altro"); d.wait_for_selector(".tmenu", state="visible")
            b = d.locator(".tmenu").bounding_box(); assert b["x"] >= 0 and b["x"] + b["width"] <= 390, b
            d.tap(".tmenu >> text=Tavola periodica"); d.wait_for_function("TEORIA.strumenti.aperto()==='tavola'")
            c.close()
        step("phone: search, gear, «Altro» with the tools, info kept, targets >= 40 px", phone)
        d = new(); d.goto(T + "13-formula.html"); d.wait_for_selector("#tbar")
        def axe():
            if not AXE: print("SKIP axe (pip install axe-playwright-python)"); return
            d.add_script_tag(path=AXE)
            v = d.evaluate("axe.run('#top',{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes[0].html.slice(0,90)]))")
            assert not v, v
            d.click("#tc-strumenti"); v = d.evaluate("axe.run('#top',{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes[0].html.slice(0,90)]))")
            assert not v, v
        step("axe on the header (also with the menu open)", axe)
        d.close()
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad else 0)
