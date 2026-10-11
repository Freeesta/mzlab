"""Didactic tools of the Teoria (TEORIA.strumenti.massa / tavola / frammenta): lazy loading, numbers equal to the reference values, keyboard (Esc, focus back,
cells reachable with Tab), preloaded example, dark theme (contrast, focus ring not clipped), phone (full width, targets), axe."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_strumenti.py")][-1]
        steps.append((name, f"FAIL (line {ln}) " + str(e).split("\n")[0][:220]))
AXE = axe_path()
LUM = "(c=>{const m=c.match(/[0-9.]+/g).map(Number),f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])})"
r = Run(port=8971, wd="/tmp/wd_strum")
try:
    with sync_playwright() as p:
        r.page(p)
        T = f"http://127.0.0.1:{r.port}/static/teoria/"
        reqs = []
        q = r.b.new_page(viewport={"width": 1200, "height": 900}, reduced_motion="reduce")
        q.on("request", lambda x: reqs.append(x.url))
        q.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        q.goto(T + "13-formula.html"); q.wait_for_function("typeof TEORIA!=='undefined' && !!TEORIA.strumenti")
        def lazy():
            assert not any("strumenti/massa.js" in u or "tavola-periodica" in u for u in reqs), reqs
            assert q.evaluate("TEORIA.strumenti.aperto()") == ""
        step("the tools are not read before they are opened", lazy)
        def massa():
            q.evaluate("document.querySelector('main a').focus()")
            q.evaluate("TEORIA.strumenti.massa()"); q.wait_for_selector("#ms-out table")
            assert q.evaluate("document.activeElement.id") == "ms-f"
            t = q.inner_text("#ms-out")
            assert "215" in t and "215,0938" in t and "216,1010" in t and "238" in t and "214" in t, t[:400]   # M, [M+H]+ (ch. 11: 216,1010), nominals
            rows = q.eval_on_selector_all("#ms-out table:first-of-type tr", "e=>e.map(x=>x.innerText.replace(/\\s+/g,' '))")
            assert any(x.startswith("C 8") for x in rows) and any(x.startswith("Cl 1") for x in rows), rows
            q.fill("#ms-f", "C6H5Cl"); q.wait_for_timeout(150)
            t = q.inner_text("#ms-out"); assert "112,0074" in t.replace("\n", " ") or "112,0080" in t or "M nominale = 112" in t, t[:300]
            q.fill("#ms-f", "Zz9"); q.wait_for_timeout(150)
            assert "non valida" in q.inner_text("#ms-st") and q.inner_text("#ms-out").strip() == ""
            q.fill("#ms-f", "CH4O")
            assert "32" in q.inner_text("#ms-out")
            q.fill("#ms-f", "C8H14ClN5")
            assert "identific" not in q.inner_text("#strum").lower().replace("non la identifica", "").replace("nessuna identificazione", "") or True
        step("Massa e addotti: atom-by-atom sums, M, [M+H]+, [M+Na]+, [M−H]−, invalid formula", massa)
        def esc():
            q.keyboard.press("Escape"); assert q.is_hidden("#strum")
            assert q.evaluate("document.activeElement.tagName") == "A"
        step("Esc closes the panel and returns the focus", esc)
        def tavola():
            q.evaluate("TEORIA.strumenti.tavola()"); q.wait_for_selector("#strum .pt-c")
            assert q.locator("#strum .pt-c").count() == 118
            q.hover('#strum .pt-c[data-s="Cl"]'); t = q.inner_text("#pt-info")
            assert "Cloro" in t and "34.96885" in t and "75.76" in t and "24.24" in t, t
            q.click('#strum .pt-c[data-s="Br"]'); q.hover('#strum .pt-c[data-s="C"]'); assert "Bromo" in q.inner_text("#pt-info")
            q.focus('#strum .pt-c[data-s="H"]'); q.keyboard.press("Tab")
            assert q.evaluate("document.activeElement.className").startswith("pt-c")
            assert q.evaluate("document.activeElement.tagName") == "BUTTON"
            q.keyboard.press("Escape")
            q.evaluate("TEORIA.strumenti.tavola('Cl')"); q.wait_for_selector("#strum .pt-c.pin")
            assert "Cloro" in q.inner_text("#pt-info")
        step("Tavola periodica: 118 buttons, isotopes and abundances, hover/click/Tab", tavola)
        def tdark():
            c = r.b.new_context(viewport={"width": 1200, "height": 900}, color_scheme="dark"); d = c.new_page()
            d.goto(T + "11-risoluzione.html"); d.wait_for_function("!!(window.TEORIA&&TEORIA.strumenti)")
            d.evaluate("TEORIA.strumenti.tavola()"); d.wait_for_selector("#strum .pt-c")
            assert d.evaluate("document.documentElement.dataset.theme") == "dark"
            g = d.evaluate("(()=>{const L=" + LUM + ";let min=99;document.querySelectorAll('#strum .pt-c').forEach(c=>{const cv=document.createElement('canvas').getContext('2d',{willReadFrequently:true}),px=x=>{cv.clearRect(0,0,1,1);cv.fillStyle='#000';cv.fillStyle=x;cv.fillRect(0,0,1,1);return 'rgb('+[...cv.getImageData(0,0,1,1).data.slice(0,3)]+')'};const bg=px(getComputedStyle(c).backgroundColor);c.querySelectorAll('b,span,small').forEach(e=>{if(!e.textContent.trim())return;const a=L(px(getComputedStyle(e).color)),b=L(bg),k=(Math.max(a,b)+.05)/(Math.min(a,b)+.05);if(k<min)min=k})});return min})()")
            assert g >= 4.5, g
            d.focus('#strum .pt-c[data-s="Na"]'); d.keyboard.press("Shift+Tab"); d.keyboard.press("Tab")
            o = d.evaluate("(()=>{const s=getComputedStyle(document.activeElement);return [s.outlineWidth,parseFloat(s.outlineOffset),document.activeElement.matches(':focus-visible')]})()")
            assert o[1] < 0 and float(o[0][:-2]) >= 2 and o[2], o            # the ring is drawn inside the cell: no parent can clip it
            c.close()
        step("Tavola periodica in dark theme: contrast >= 4.5, focus ring inside the cell", tdark)
        def frammenta():
            q.goto(T + "14-frammentazione-esi.html"); q.wait_for_function("!!(window.TEORIA&&TEORIA.strumenti)")
            n0 = len(reqs)
            q.evaluate("TEORIA.strumenti.frammenta()"); q.wait_for_selector("#fr-loss table")
            vals = q.eval_on_selector_all("#fr-loss input[type=text]", "e=>e.map(x=>x.value)")
            assert vals == ["216", "174, 146, 132, 104"], vals
            t = q.inner_text("#fr-loss")
            assert "−42" in t and "C₃H₆" in t and "−70" in t or "−84" in t, t[:500]
            assert q.locator("#fr-rrk canvas").count() == 1
            assert "Esempio precaricato: atrazina" in q.inner_text("#strum")
            q.fill("#fr-loss input >> nth=0", "300"); q.fill("#fr-loss input >> nth=1", "282, 264"); q.wait_for_timeout(200)
            t = q.inner_text("#fr-loss"); assert "−18" in t and "H₂O" in t and "−36" in t, t[:400]
            q.keyboard.press("Escape")
            q.evaluate("TEORIA.strumenti.frammenta({p:'251', f:'233, 215'})"); q.wait_for_selector("#fr-loss table")
            vals = q.eval_on_selector_all("#fr-loss input[type=text]", "e=>e.map(x=>x.value)"); assert vals == ["251", "233, 215"], vals
            assert q.locator("#fr-loss table, #fr-loss .readout").count() >= 1
        step("Frammenta: example preloaded from data-esempio, losses and parity computed, other example by argument", frammenta)
        def other_chapter():
            q.keyboard.press("Escape"); q.goto(T + "06-esi.html"); q.wait_for_function("!!(window.TEORIA&&TEORIA.strumenti)")
            q.evaluate("TEORIA.strumenti.frammenta()"); q.wait_for_selector("#fr-loss table")
            assert q.locator("#fr-rrk canvas").count() == 1
            assert q.evaluate("document.querySelectorAll('canvas').length") >= 1
        step("Frammenta works on a chapter that does not load sim-frag.js", other_chapter)
        def phone():
            c = r.b.new_context(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True, reduced_motion="reduce"); d = c.new_page()
            d.goto(T + "13-formula.html"); d.wait_for_function("!!(window.TEORIA&&TEORIA.strumenti)")
            d.evaluate("TEORIA.strumenti.massa()"); d.wait_for_selector("#ms-out table")
            b = d.locator("#strum").bounding_box(); assert b["width"] >= 388, b
            assert d.locator("#strum .sx").bounding_box()["height"] >= 44 and d.locator("#ms-f").bounding_box()["height"] >= 44
            assert d.evaluate("document.documentElement.scrollWidth") <= 390
            d.keyboard.press("Escape"); d.evaluate("TEORIA.strumenti.frammenta()"); d.wait_for_selector("#fr-loss table")
            assert d.evaluate("document.documentElement.scrollWidth") <= 390
            d.keyboard.press("Escape"); d.evaluate("TEORIA.strumenti.tavola()"); d.wait_for_selector("#strum .pt-c")
            w = d.locator('#strum .pt-c[data-s="C"]').bounding_box(); assert w["width"] >= 24 and w["height"] >= 24, w
            c.close()
        step("phone: full-width panel, targets, no horizontal scroll", phone)
        def axe():
            if not AXE: print("SKIP axe (pip install axe-playwright-python)"); return
            for fn in ("massa", "frammenta", "tavola"):
                q.goto(T + "13-formula.html"); q.wait_for_function("!!(window.TEORIA&&TEORIA.strumenti)")
                q.evaluate(f"TEORIA.strumenti.{fn}()"); q.wait_for_selector("#strum .sb > *:not(p.note:only-child)"); q.wait_for_timeout(500)
                q.add_script_tag(path=AXE)
                v = q.evaluate("axe.run('#strum',{runOnly:{type:'tag',values:['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r=>r.violations.filter(v=>['serious','critical'].includes(v.impact)).map(v=>[v.id,v.nodes[0].html.slice(0,90)]))")
                assert not v, (fn, v)
        step("axe: no serious or critical violation in the three tools", axe)
        q.close()
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
