"""Search in the Teoria (cerca.js): Ctrl/⌘K and TEORIA.cerca() open a combobox dialog, the index is read only on first use, «frammenta» finds the
fragmentation entries grouped by chapter, arrows/Enter/Esc, Enter goes to the anchor, focus trapped and returned, dark theme, phone (44 px, no scroll), axe."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_cerca.py")][-1]
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
r = Run(port=8970, wd="/tmp/wd_cerca")
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        reqs = []
        q = r.b.new_page(viewport={"width": 1200, "height": 900}, reduced_motion="reduce")
        q.on("request", lambda x: reqs.append(x.url))
        q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        q.goto(T + "13-formula.html"); q.wait_for_selector("#side a")
        q.wait_for_function("typeof TEORIA!=='undefined' && typeof TEORIA.cerca==='function'")
        def lazy():
            assert not any("indice-ricerca" in u for u in reqs), "index read before the first opening"
            assert q.is_hidden("#cerca-bg") if q.query_selector("#cerca-bg") else True
        step("the index is not read before the first opening", lazy)
        def opens():
            q.evaluate("document.querySelector('main a').focus()")
            q.keyboard.press("Control+k"); q.wait_for_selector("#cerca-in", state="visible")
            assert q.evaluate("document.activeElement.id") == "cerca-in"
            a = q.evaluate("(()=>{const i=document.querySelector('#cerca-in');return [i.getAttribute('role'),i.getAttribute('aria-controls'),document.querySelector('#cerca').getAttribute('role'),document.querySelector('#cerca').getAttribute('aria-modal'),document.querySelector('#cerca-list').getAttribute('role')]})()")
            assert a == ["combobox", "cerca-list", "dialog", "true", "listbox"], a
            q.wait_for_function("typeof INDICE_RICERCA!=='undefined'")
            assert sum("indice-ricerca" in u for u in reqs) == 1
        step("Ctrl+K opens the combobox dialog; index loaded on first use", opens)
        def search():
            q.fill("#cerca-in", "frammenta"); q.wait_for_selector("#cerca-list [role=option]")
            txt = q.eval_on_selector_all("#cerca-list [role=option]", "e=>e.map(x=>x.innerText)")
            assert any("Frammentazione in CID" in t for t in txt), txt[:5]
            groups = q.eval_on_selector_all("#cerca-list .gh", "e=>e.map(x=>x.textContent)")
            assert "Cap. 14" in groups and len(groups) == len(set(groups)), groups
            assert q.evaluate("document.querySelectorAll('#cerca-list mark').length") >= 1
            assert q.evaluate("document.querySelector('#cerca-in').getAttribute('aria-expanded')") == "true"
            assert "risultat" in q.inner_text("#cerca-st")
            q.fill("#cerca-in", "FRAMMENTAZIONE È"); q.wait_for_timeout(150)   # accents and case do not matter
            assert q.locator("#cerca-list [role=option]").count() >= 1
        step("«frammenta» finds the fragmentation entries, grouped by chapter, highlighted", search)
        def synonym():
            q.fill("#cerca-in", "neutral loss"); q.wait_for_timeout(200)
            assert any("erdit" in t for t in q.eval_on_selector_all("#cerca-list [role=option]", "e=>e.map(x=>x.innerText)"))
            q.fill("#cerca-in", "zzzzqq"); q.wait_for_timeout(200)
            assert q.locator("#cerca-list [role=option]").count() == 0 and "Nessun" in q.inner_text("#cerca-st")
        step("synonyms (neutral loss) and no-result message", synonym)
        def keys():
            q.fill("#cerca-in", "addott"); q.wait_for_selector("#cerca-list [role=option]")
            n = q.locator("#cerca-list [role=option]").count(); assert n >= 2
            assert q.get_attribute("#co0", "aria-selected") == "true"
            q.keyboard.press("ArrowDown")
            assert q.get_attribute("#co1", "aria-selected") == "true" and q.get_attribute("#cerca-in", "aria-activedescendant") == "co1"
            q.keyboard.press("ArrowUp"); q.keyboard.press("ArrowUp")
            assert q.get_attribute(f"#co{n-1}", "aria-selected") == "true"        # wraps
            q.keyboard.press("Tab"); assert q.evaluate("document.activeElement.className") == "cx"
            q.keyboard.press("Tab"); assert q.evaluate("document.activeElement.id") == "cerca-in"   # trapped
            q.keyboard.press("Shift+Tab"); assert q.evaluate("document.activeElement.className") == "cx"
            q.keyboard.press("Tab")
        step("arrows (wrap), Tab trapped in the dialog", keys)
        def esc():
            q.keyboard.press("Escape"); assert q.is_hidden("#cerca-bg")
            assert q.evaluate("document.activeElement.tagName") == "A", "focus not returned"
        step("Esc closes and returns the focus", esc)
        def go():
            q.evaluate("TEORIA.cerca('regola dell')"); q.wait_for_selector("#cerca-list [role=option]")
            q.keyboard.press("Enter"); q.wait_for_timeout(400)
            assert q.is_hidden("#cerca-bg")
            h = q.evaluate("location.hash"); assert h, "no anchor"
            tgt = q.evaluate("(()=>{const e=document.getElementById(location.hash.slice(1));if(!e)return null;const b=e.getBoundingClientRect();return [e.textContent.slice(0,40),b.top]})()")
            assert tgt and tgt[1] < 400, tgt
        step("TEORIA.cerca(q) + Enter goes to the anchor on the same page", go)
        def other():
            q.keyboard.press("Control+k"); q.fill("#cerca-in", "atrazina"); q.wait_for_timeout(300)
            q.fill("#cerca-in", "glossario"); q.wait_for_selector("#cerca-list [role=option]")
            q.locator("#cerca-list [role=option]").first.click(); q.wait_for_load_state(); q.wait_for_timeout(500)
            assert q.url.startswith(T), q.url
        step("a click on a result in another chapter navigates", other)
        def gl():
            q.goto(T + "13-formula.html"); q.wait_for_function("typeof TEORIA!=='undefined'")
            q.evaluate("TEORIA.cerca('addotto')"); q.wait_for_selector("#cerca-list [role=option]")
            idx = q.eval_on_selector_all("#cerca-list [role=option]", "e=>e.map(x=>x.innerText)")
            i = next(k for k, t in enumerate(idx) if t.endswith("Glossario")); 
            for _ in range(i): q.keyboard.press("ArrowDown")
            q.keyboard.press("Enter"); q.wait_for_load_state(); q.wait_for_timeout(600)
            assert "21-glossario.html#g-" in q.url, q.url
            assert q.evaluate("document.getElementById(location.hash.slice(1)).tagName") == "DT"
        step("a glossary entry lands on its term", gl)
        def dark():
            c = r.b.new_context(viewport={"width": 1200, "height": 900}, color_scheme="dark"); d = c.new_page()
            d.goto(T + "04-lc.html"); d.wait_for_function("typeof TEORIA!=='undefined'")
            d.evaluate("TEORIA.cerca('gradiente')"); d.wait_for_selector("#cerca-list [role=option]")
            L = "(c=>{const m=c.match(/[0-9.]+/g).map(Number),f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])})"
            ratio = d.evaluate(f"(()=>{{const L={L};const a=L(getComputedStyle(document.querySelector('#cerca')).color),b=L(getComputedStyle(document.querySelector('#cerca')).backgroundColor);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)}})()")
            assert ratio > 7, ratio
            assert d.evaluate("getComputedStyle(document.querySelector('#cerca')).backgroundColor") != "rgb(255, 255, 255)"
            m = d.evaluate("(()=>{const L=" + L + ";const a=L(getComputedStyle(document.querySelector('#cerca-list mark')).color),b=L(getComputedStyle(document.querySelector('#cerca')).backgroundColor);return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)})()")
            assert m >= 4.5, m
            c.close()
        step("dark theme: readable dialog and highlight", dark)
        def phone():
            c = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, reduced_motion="reduce"); d = c.new_page()
            d.goto(T + "06-esi.html"); d.wait_for_function("typeof TEORIA!=='undefined'")
            d.evaluate("TEORIA.cerca('esi')"); d.wait_for_selector("#cerca-list [role=option]")
            hs = d.eval_on_selector_all("#cerca-list [role=option]", "e=>e.map(x=>x.getBoundingClientRect().height)")
            assert min(hs) >= 44, min(hs)
            for s in ("#cerca-in", "#cerca .cx"): assert d.locator(s).bounding_box()["height"] >= 44
            assert d.evaluate("document.documentElement.scrollWidth") <= 390
            b = d.locator("#cerca").bounding_box(); assert b["x"] >= 0 and b["x"] + b["width"] <= 390, b
            d.locator("#cerca-list [role=option]").nth(1).tap(); d.wait_for_timeout(500)
            assert d.is_hidden("#cerca-bg")
            c.close()
        step("phone: 390 px, targets >= 44 px, a tap on a result navigates", phone)
        def axe():
            if not AXE: print("SKIP axe (pip install axe-playwright-python)"); return
            q.goto(T + "13-formula.html"); q.wait_for_function("typeof TEORIA!=='undefined'")
            q.evaluate("TEORIA.cerca('frammenta')"); q.wait_for_selector("#cerca-list [role=option]")
            q.add_script_tag(path=AXE)
            v = q.evaluate("axe.run('#cerca-bg',{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes[0].html.slice(0,80)]))")
            assert not v, v
        step("axe: no serious or critical violation", axe)
        q.close()
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad else 0)
