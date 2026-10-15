"""Commands of the Teoria (comandi.js, TCMD): the header carries only index toggle, previous/next and the gear; the side panel (#sidecmd) carries search, glossary, formulas,
exercises, download and the tools (nothing is lost); labels that give way when the row is narrow, narrow screens (previous/next in the fixed bar, the commands in the index sheet), phone, targets, axe."""
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
            assert ids == ["tc-indice", "tc-prec", "tc-succ", "a11y-btn"], ids
            side = q.eval_on_selector_all("#sidecmd > *", "e=>e.map(x=>x.id)"); assert side == ["sc-cerca", "sc-glossario", "sc-formule", "sc-esercizi", "sc-esporta", "sc-strumenti"], side
            names = q.eval_on_selector_all("#tbar a,#tbar button", "e=>e.map(x=>x.getAttribute('aria-label'))")
            assert names[0].startswith("Indice") and names[-1].startswith("Impostazioni"), names
            for sel in ("#sc-cerca", "#sc-glossario", "#sc-formule", "#sc-esercizi", "#sc-esporta", "#tc-indice", "#tc-prec"):
                lab = q.inner_text(sel).strip(); assert lab and lab in q.get_attribute(sel, "aria-label"), (sel, lab)   # visible label in the name (WCAG 2.5.3)
            assert q.get_attribute("#tc-indice", "aria-pressed") == "true"
            for dead in ("Esporta capitolo o corso", "Avanzamento"): assert dead not in q.inner_text("#top") + q.inner_text("#sidecmd"), dead
            for t in ("Dati", "Disegno", "Teoria"): assert q.is_visible(f"#tabs >> text={t}")
            for dead in ("Calculator", "Losses", "Adducts", "Lists", "Table", "More", "Search", "New session", "Segnalibri", "Nuova sessione"): assert not q.query_selector(f"#tbar >> text='{dead}'"), dead
        step("order, names, labels in the names, tabs kept, no data or discarded commands", order)
        def targets():
            for s in ("#tc-indice", "#tc-prec", "#tc-succ", "#a11y-btn"):
                b = q.locator(s).bounding_box(); assert b["height"] >= 44 and b["width"] >= 44, (s, b)
            for s in ("#sc-cerca", "#sc-glossario", "#sc-formule", "#sc-esercizi", "#sc-esporta", "#sc-strumenti summary"):
                b = q.locator(s).bounding_box(); assert b["height"] >= 24 and b["width"] >= 24, (s, b)
        step("targets: header at least 44x44 px, side commands at least 24 px", targets)
        def prevnext():
            n = q.get_attribute("#tc-succ", "href"); assert n == "14-frammentazione-esi.html", n
            q.click("#tc-succ"); q.wait_for_url("**/14-frammentazione-esi.html")
            q.wait_for_selector("#tbar"); assert q.get_attribute("#tc-prec", "href") == "13-formula.html"
            q.click("#tc-prec"); q.wait_for_url("**/13-formula.html"); q.wait_for_selector("#tbar")
            q.goto(T + "index.html"); q.wait_for_selector("#tbar"); assert q.get_attribute("#tc-prec", "aria-disabled") == "true" and q.get_attribute("#tc-prec", "href") is None
            q.goto(T + "13-formula.html"); q.wait_for_selector("#tbar")
        step("previous / next change the chapter, the first has no previous", prevnext)
        def cerca():
            q.click("#sc-cerca"); q.wait_for_selector("#cerca-in", state="visible"); assert q.evaluate("TEORIA.cerca.aperta()")
            q.keyboard.press("Escape"); q.wait_for_selector("#cerca-bg", state="hidden")
            assert q.evaluate("document.activeElement.id") == "sc-cerca"
            q.keyboard.press("Control+k"); q.wait_for_selector("#cerca-in", state="visible"); q.keyboard.press("Escape")
        step("Cerca opens the dialog (button and Ctrl+K), Esc gives the focus back", cerca)
        def links():
            q.click("#sc-glossario"); q.wait_for_url("**/21-glossario.html"); q.wait_for_selector("#tbar")
            q.click("#sc-formule"); q.wait_for_url("**/22-formulario.html"); q.wait_for_selector("#tbar"); q.goto(T + "13-formula.html"); q.wait_for_selector("#tbar")
        step("Glossario and Formule go to the chapters 21 and 22", links)
        def esercizi():
            q.goto(T + "16-ei-famiglie.html"); q.wait_for_selector("#tbar"); q.click("#sc-esercizi"); q.wait_for_timeout(300)
            assert q.evaluate("document.activeElement.classList.contains('gym')"), q.evaluate("document.activeElement.outerHTML.slice(0,80)")
            q.goto(T + "04-lc.html"); q.wait_for_selector("#tbar"); q.click("#sc-esercizi"); q.wait_for_timeout(300)
            assert q.evaluate("/^Verifica/.test(document.activeElement.textContent.trim())")
            q.goto(T + "pratica-ei.html"); q.wait_for_selector("#tbar"); q.click("#sc-esercizi")
        step("Esercizi goes to the «Allenati» box, else «Verifica»", esercizi)
        def strum():
            q.goto(T + "14-frammentazione-esi.html"); q.wait_for_selector("#sidecmd"); q.wait_for_function("typeof TEORIA.strumenti==='object'")
            assert q.eval_on_selector("#sc-strumenti", "e=>e.open") is False
            q.focus("#sc-strumenti summary"); q.keyboard.press("Enter"); assert q.eval_on_selector("#sc-strumenti", "e=>e.open")
            items = q.eval_on_selector_all("#sc-strumenti .sc-sub button", "e=>e.map(x=>x.textContent)"); assert items == ["Frammenta", "Massa e addotti", "Tavola periodica", "Perdite"], items
            q.click("#sc-strumenti [data-cmd=tavola]"); q.wait_for_function("TEORIA.strumenti.aperto()==='tavola'")
            q.keyboard.press("Escape"); q.wait_for_function("TEORIA.strumenti.aperto()===''")
            q.click("#sc-strumenti [data-cmd=massa]"); q.wait_for_function("TEORIA.strumenti.aperto()==='massa'"); q.evaluate("TEORIA.strumenti.chiudi()")
            q.click("#sc-strumenti [data-cmd=frammenta]"); q.wait_for_function("TEORIA.strumenti.aperto()==='frammenta'"); q.evaluate("TEORIA.strumenti.chiudi()")
        step("Strumenti in the side panel: the four entries open the tools", strum)
        def scarica():
            q.click("#sc-esporta"); q.wait_for_selector("#export-modal", state="visible")
            heads = q.eval_on_selector_all("#export-modal thead th", "e=>e.map(x=>x.textContent.trim())"); assert heads[1:] == ["Capitolo", "Tutta la Teoria"], heads
            for k in ("pdf", "md", "doc", "html"):
                assert q.get_attribute(f"#exp-cur-{k}", "aria-label").startswith("Capitolo, ") and q.get_attribute(f"#exp-all-{k}", "aria-label").startswith("Tutta la Teoria, ")
            assert q.locator("#export-modal tbody tr").count() == 4 and "Pacchetto" not in q.inner_text("#export-modal")
            with q.expect_download() as dl: q.click("#exp-cur-md")
            assert dl.value.suggested_filename.endswith(".md")
            q.keyboard.press("Escape"); q.wait_for_selector("#export-modal", state="hidden")
        step("Scarica: one row per format, two columns, the chapter downloads", scarica)
        def indice():
            q.click("#tc-indice"); assert q.is_hidden("#side") and q.get_attribute("#tc-indice", "aria-pressed") == "false"
            q.click("#tc-indice"); assert q.is_visible("#side") and q.is_visible("#sc-cerca")
        step("the index toggle hides and shows the side panel with its commands", indice)
        def gear():
            q.click("#a11y-btn"); assert q.is_visible("#a11y-panel"); q.keyboard.press("Escape"); assert q.is_hidden("#a11y-panel")
        step("the gear opens the reading settings", gear)
        def fit():
            q.set_viewport_size({"width": 1000, "height": 800}); q.wait_for_timeout(300)
            assert q.evaluate("document.querySelector('#top').scrollWidth<=document.querySelector('#top').clientWidth")
            q.set_viewport_size({"width": 1280, "height": 900}); q.wait_for_timeout(300)
            assert q.is_visible("#tc-prec .l")
        step("labels give way when the row is narrow, no overflow", fit)
        q.close()
        def narrow():
            d = new(820, 900); page(d, "13-formula.html")
            assert d.is_visible("#cb-succ") and d.is_visible("#cb-prec") and d.is_visible("#menu") and d.is_visible("#a11y-btn")
            assert not d.is_visible("#tc-prec") and not d.is_visible("#tc-indice") and not d.is_visible("#sc-cerca")
            for s in ("#cb-prec", "#cb-succ", "#menu", "#a11y-btn"):
                b = d.locator(s).bounding_box(); assert b["height"] >= 44 and b["width"] >= 44, (s, b)
            assert d.evaluate("document.documentElement.scrollWidth") <= 820
            d.click("#menu"); d.wait_for_selector("#side.open #sc-cerca", state="visible")
            for s in ("#sc-cerca", "#sc-esporta", "#sc-strumenti summary"):
                b = d.locator(s).bounding_box(); assert b["height"] >= 40, (s, b)
            d.click("#side .sheetx"); d.click("#cb-succ"); d.wait_for_url("**/14-frammentazione-esi.html"); d.close()
        step("<= 980 px: previous/next in the fixed bar, the commands in the index sheet", narrow)
        def phone():
            c = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, reduced_motion="reduce"); d = c.new_page()
            d.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            page(d, "06-esi.html")
            assert d.is_visible("#a11y-btn") and d.is_visible("#info") and d.is_visible("#menu") and d.locator("#tc-altro").count() == 0
            for s in ("#a11y-btn", "#info", "#menu"):
                b = d.locator(s).bounding_box(); assert b["height"] >= 40 and b["width"] >= 40, (s, b)
            assert d.evaluate("document.documentElement.scrollWidth") <= 390
            d.tap("#menu"); d.wait_for_selector("#side.open #sc-cerca", state="visible")
            d.tap("#sc-strumenti summary"); d.tap("#sc-strumenti [data-cmd=tavola]"); d.wait_for_function("TEORIA.strumenti.aperto()==='tavola'")
            c.close()
        step("phone: gear and info at the top, search and tools in the index sheet, targets >= 40 px", phone)
        d = new(); d.goto(T + "13-formula.html"); d.wait_for_selector("#tbar")
        def axe():
            if not AXE: print("SKIP axe (pip install axe-playwright-python)"); return
            d.add_script_tag(path=AXE)
            v = d.evaluate("axe.run('#top',{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes[0].html.slice(0,90)]))")
            assert not v, v
            v = d.evaluate("axe.run('#sidecmd',{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes[0].html.slice(0,90)]))")
            assert not v, v
        step("axe on the header and on the side commands", axe)
        d.close()
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad else 0)
