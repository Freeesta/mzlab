"""Block C: the m/z calculator is a drop-down under its button (no dark backdrop, graphs stay usable); lower-case formulas; no '1 decimale' column or offset sentence."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def until(pg, js, t=10000): pg.wait_for_function(js, timeout=t)       # wait for the state the next assertion reads
SUM = "document.querySelector('#calcsum').innerText.replace(/ /g, '')"
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_calc.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8879, wd="/tmp/wd79")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")])
        pg.click("text=Carica dati"); ready(pg)
        def open_below():
            pg.click("#np-calc2"); attendi(pg, "window.BARRA.curTab === 'calc' && !document.querySelector('#sb-panel-calc').hidden")
            assert pg.evaluate("window.BARRA.curTab === 'calc' && !document.querySelector('#sb-panel-calc').hidden")
            # In Disegno view, np-calc2 opens floating dropdown right under its button, not modal, >= 520px wide
            pg.click("#nav [data-v=draw]"); frames(pg, 3)
            pg.click("#np-calc2"); attendi(pg, "!document.querySelector('#calcdlg').hidden"); frames(pg, 3)
            b = pg.evaluate("(()=>{const a=document.querySelector('#np-calc2').getBoundingClientRect(),c=document.querySelector('#calcdlg');const r=c.getBoundingClientRect();return {ab:a.bottom,ct:r.top,w:r.width,vis:!c.hidden,modal:c.matches(':modal')}})()")
            assert b["vis"] and not b["modal"] and 0 < b["ct"] - b["ab"] < 20 and b["w"] >= 520, b
            pg.click("#calcx"); attendi(pg, "document.querySelector('#calcdlg').hidden")
            pg.click("#nav [data-v=data]"); frames(pg, 3)
            if not pg.evaluate("!!document.querySelector('#calcin').offsetParent"):    # the sidebar tab stays open across views: the button is a toggle
                pg.click("#np-calc2"); attendi(pg, "!!document.querySelector('#calcin').offsetParent")
        step("in Dati opens sidebar tab, in Disegno opens right under its button >= 520 px wide", open_below)
        def usable():
            frames(pg, 3)               # docked next to the charts the calculator stays open while the student works on them
            c = pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='chrom'&&q.tab==='full'),r=p.cv.getBoundingClientRect();return {x:r.left+p._a.X(14.3),y:r.bottom-60}})()")
            pg.evaluate("window.scrollTo(0,0)")
            pg.mouse.click(c["x"], c["y"] if c["y"] < 700 else 400); attendi(pg, "E.panels.find(q=>q.type==='chrom'&&q.tab==='full').cur != null"); frames(pg, 3)
            assert pg.evaluate("document.querySelector('#calcdlg').hidden") is False, "stays open while the student works on the graphs"
            assert pg.evaluate("E.panels.find(q=>q.type==='chrom'&&q.tab==='full').cur") is not None
        step("the graphs stay usable with the calculator open", usable)
        def lower():
            pg.fill("#calcin", "c14h13f4n3o2s"); until(pg, "document.querySelector('#calcsum').innerText.includes('interpretata come')")
            assert "interpretata come" in pg.inner_text("#calcsum") and "C14H13F4N3O2S" in pg.inner_text("#calcsum").replace(" ", "")
            t = pg.inner_text("#calcout"); assert "364.1" in t and "364.07" not in t and "1 decimale" not in t and "spostato di qualche decimo" not in t, t
            pg.fill("#calcin", "C9h10Cl2n2O"); until(pg, SUM + ".includes('C9H10Cl2N2O')"); assert "C9H10Cl2N2O" in pg.inner_text("#calcsum").replace(" ", "")
            pg.fill("#calcin", "co"); until(pg, SUM + ".includes('CO')"); assert "CO" in pg.inner_text("#calcsum"), pg.inner_text("#calcsum")
        step("lower-case formula: interpreted line, no extra column / sentence", lower)
        def arithmetic():
            pg.click("#np-calc2") if pg.evaluate("document.querySelector('#calcdlg').hidden") else None
            pg.fill("#calcin", "364.4-194.2"); until(pg, "document.querySelector('#calcres').innerText.trim() === '170.2'")
            assert pg.inner_text("#calcres").strip() == "170.2" and not pg.evaluate("document.querySelector('#calcpad').hidden")
            assert pg.is_visible("#calc-nl") and "170.2" in pg.get_attribute("#calc-nl", "title")
            pg.fill("#calcin", "364,4 \u2212 194,2"); until(pg, "document.querySelector('#calcres').innerText.trim() === '170.2'"); assert pg.inner_text("#calcres").strip() == "170.2", pg.inner_text("#calcres")
            pg.fill("#calcin", "(229.1-171.2)*2"); until(pg, "document.querySelector('#calcres').innerText.trim() === '115.8'"); assert pg.inner_text("#calcres").strip() == "115.8"
            pg.click("#calc-nl"); until(pg, "window.BARRA.curTab === 'losses' && !document.querySelector('#sb-panel-losses').hidden")
            assert pg.evaluate("window.BARRA.curTab === 'losses' && !document.querySelector('#sb-panel-losses').hidden")
            assert pg.input_value("#nl-q") == "115.8"
            pg.click("#np-calc2"); attendi(pg, "window.BARRA.curTab === 'calc'")
        step("a calculation: 364.4-194.2 = 170.2 (also with comma and the minus sign), calc-nl button opens losses tab", arithmetic)
        def keypad():
            pg.fill("#calcin", ""); frames(pg, 2)
            for k in ("1", "+", "2"): pg.click(f"#calcpad [data-k='{k}']")
            assert pg.inner_text("#calcres").strip() == "3"
            pg.click("#calcpad [data-k='=']"); until(pg, "document.querySelector('#calcin').value === '3'")
            assert pg.input_value("#calcin") == "3" and pg.locator("#calctape div").count() == 1 and "1 + 2" in pg.inner_text("#calctape")
        step("keypad: 1 + 2 = 3 and the tape", keypad)
        def keyboard():
            pg.fill("#calcin", ""); pg.click("#calcin"); pg.keyboard.type("5*4"); pg.keyboard.press("Enter"); until(pg, "document.querySelector('#calcin').value === '20'")
            assert pg.input_value("#calcin") == "20" and pg.locator("#calctape div").count() == 2, (pg.input_value("#calcin"), pg.locator("#calctape div").count())
            pg.keyboard.type("5"); assert pg.input_value("#calcin") == "5", "a digit after = starts a new calculation"
            pg.fill("#calcin", "Ans+1"); pg.keyboard.press("Enter"); until(pg, "document.querySelector('#calcin').value === '21'"); assert pg.input_value("#calcin") == "21", pg.input_value("#calcin")
            pg.locator("#calctape div").nth(2).click(); until(pg, "document.querySelector('#calcin').value === '3'"); assert pg.input_value("#calcin") == "3", "a row of the tape puts its result back in the field"
            pg.keyboard.press("Escape"); until(pg, "document.querySelector('#calcin').value === ''"); assert pg.input_value("#calcin") == "" and not pg.evaluate("document.querySelector('#calcdlg').hidden"), "Esc clears first"
        step("keyboard: Enter = result, a new digit starts again, Ans, tape row, Esc clears", keyboard)
        def formula_hides_pad():
            pg.fill("#calcin", "C9H10Cl2N2O"); until(pg, "document.querySelector('#calcpad').hidden && document.querySelectorAll('#calcout .cadd-wrap table').length === 1")      # the table is drawn after the pad is hidden
            assert pg.evaluate("document.querySelector('#calcpad').hidden") and "C9H10Cl2N2O" in pg.inner_text("#calcout").replace(" ", "") and pg.locator("#calcout .cadd-wrap table").count() == 1
            assert pg.evaluate("document.querySelector('#calcres').hidden")
            pg.fill("#calcin", ""); until(pg, "!document.querySelector('#calcpad').hidden")
        step("a formula: adduct table as before, no keypad", formula_hides_pad)
        TXT = "[...document.querySelectorAll('.cadd-wrap tbody tr:not(.cmore)')].map(tr=>[...tr.children].map(c=>c.textContent.replace(/\\s+/g,' ').trim()))"
        def table_rows():
            pg.fill("#calcin", "C13H26N4"); pg.wait_for_selector(".cadd-wrap tbody tr")
            rows = pg.evaluate(TXT); names = [x[0] for x in rows]
            assert names[:4] == ["[M+H]+", "[M+NH4]+", "[M+Na]+", "[M+K]+"], names
            assert rows[0][1] == "+1.0073" and rows[0][2] == "239.2230" and rows[0][3] == "239", rows[0]
            assert [h.split()[0] for h in pg.evaluate("[...document.querySelectorAll('.cadd-wrap thead th')].map(t=>t.textContent)")][:4], "headers"
            assert pg.get_attribute(".cxic >> nth=0", "aria-label") == "XIC di [M+H]+ 239.2230", pg.get_attribute(".cxic >> nth=0", "aria-label")
            t = pg.inner_text("#calcdlg"); assert "Clicca" not in t and "Click a row" not in t and pg.locator(".adnote").count() == 0, t[:300]
        step("C13H26N4: [M+H]+ 239.2230, delta +1.0073, XIC button name, no descriptive text", table_rows)
        def polarity():
            pg.click("#calcpol button[data-p='-1']"); pg.wait_for_function("document.querySelector('.cadd-wrap tbody th').textContent.includes('[M-H]')")
            assert [x[0] for x in pg.evaluate(TXT)][:4] == ["[M-H]-", "[M+HCOO]-", "[M+CH3COO]-", "[M+Cl]-"]
            assert pg.get_attribute("#calcpol button[data-p='-1']", "aria-pressed") == "true"
            assert pg.evaluate("JSON.parse(localStorage.getItem('mzlab.calc')).pol") == -1, "polarity remembered"
            pg.click(".cadd-more"); pg.wait_for_function("document.querySelectorAll('.cadd-wrap tbody tr').length > 6")
            assert "[2M-H]-" in [x[0] for x in pg.evaluate(TXT)]
            pg.click(".cadd-more")
            pg.click("#calcpol button[data-p='1']"); pg.wait_for_function("document.querySelector('.cadd-wrap tbody th').textContent.includes('[M+H]')")
        step("ESI+/ESI- switch, remembered, More shows the rare adducts", polarity)
        def observed():
            pg.fill("#calcobs", "261.2049"); pg.wait_for_selector(".cadd-wrap tr.hit")
            assert pg.inner_text(".cadd-wrap tr.hit th").replace("\u2713", "").strip().endswith("[M+Na]+"), pg.inner_text(".cadd-wrap tr.hit")
            assert "[M+Na]+" in pg.inner_text(".cadd-match") and "Da" in pg.inner_text(".cadd-match")
            pg.fill("#calcobs", "300.1"); pg.wait_for_function("document.querySelector('.cadd-match.muted')")
            assert "5" in pg.inner_text(".cadd-match") and pg.locator(".cadd-wrap tr.hit").count() == 0
            pg.fill("#calctol", "100000"); pg.wait_for_function("!document.querySelector('.cadd-match.muted')")
            pg.fill("#calctol", "0.5"); pg.fill("#calcobs", "")
        step("observed m/z: matching row highlighted, none within the tolerance, tolerance field", observed)
        def without_formula():
            pg.fill("#calcin", ""); pg.fill("#calcobs", "239.2230"); pg.wait_for_selector(".cadd-wrap tbody tr")
            rows = pg.evaluate(TXT); assert rows[0][0] == "[M+H]+" and rows[0][-1] == "238.2157", rows[0]
            assert pg.locator(".cxic").count() == 0, "no XIC without a formula"
            pg.fill("#calcobs", "")
        step("observed m/z without a formula: M if this adduct", without_formula)
        def diffs():
            pg.fill("#calcin", "C13H26N4"); pg.wait_for_selector("#cadd-diffs")
            assert pg.get_attribute("#cadd-diffs", "open") is None, "closed by default"
            pg.click("#cadd-diffs summary"); pg.fill("#cadd-delta", "21.9819"); pg.wait_for_function("document.querySelector('#cadd-dres').textContent.includes('Na')")
            assert "Na" in pg.inner_text("#cadd-dres")
            pg.fill("#cadd-delta", "5"); pg.wait_for_function("document.querySelector('#cadd-dres .muted')")
            pg.fill("#cadd-delta", ""); pg.click("#cadd-diffs summary")
        step("mass differences: closed by default, a delta between two peaks names the adduct pair", diffs)
        def menu():
            assert pg.locator("#stbar [data-cmd=adducts]").count() == 0 and pg.locator("#np-ad").count() == 0
            pg.evaluate("BARRA.setTab('adducts')"); attendi(pg, "BARRA.curTab === 'calc'"); assert pg.evaluate("BARRA.curTab") == "calc"
            pg.evaluate("document.querySelector('#calcdlg').hidden || BARRA.setTab('pt')")
            pg.evaluate("QQQRef.open('ad')"); frames(pg, 3)
            assert pg.evaluate("!!document.querySelector('#calcin').offsetParent || !document.querySelector('#calcdlg').hidden")
        step("the ESI adducts entry opens the calculator; no dead menu entry", menu)
        def xic_click():
            pg.fill("#calcin", "C13H26N4"); pg.wait_for_selector(".cxic")
            pg.click(".cadd-wrap button.cxic >> nth=1"); pg.wait_for_selector("#xic-mz", state="visible")
            assert "NH4" in pg.input_value("#xic-ad") or "NH4" in pg.inner_text("#xic-sum") or pg.input_value("#xic-mz"), "XIC window prefilled"
            pg.click("#xic-no")
        step("XIC button opens the XIC window", xic_click)
        def composition():
            qa = pg.evaluate("fetch('api/adduct_table').then(r=>r.json()).then(t=>t.rows.map(a=>[a.n,a.add,a.sub,a.k,a.z]))")
            js = pg.evaluate("QADD.map(a=>[a.n,a.add||'',a.sub||'',a.k,a.z])"); assert js == qa, [(a, b) for a, b in zip(js, qa) if a != b][:3] or (len(js), len(qa))
        step("the composition list of the page equals the Rust catalogue", composition)
        def narrow():
            pg.click("#nav [data-v=draw]"); frames(pg, 3)
            pg.click("#np-calc2"); attendi(pg, "!document.querySelector('#calcdlg').hidden")        # the floating drop-down: same content as the sidebar tab
            pg.fill("#calcin", "C13H26N4"); pg.wait_for_selector(".cadd-wrap table")
            pg.set_viewport_size({"width": 320, "height": 800}); attendi(pg, "innerWidth === 320"); frames(pg, 3)
            try:
                assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth"), "no page-level horizontal scroll at 320 px"
                assert pg.evaluate("(()=>{const r=document.querySelector('#calcdlg').getBoundingClientRect();return r.left>=0&&r.right<=innerWidth})()"), "the calculator fits 320 px"
            finally:
                pg.set_viewport_size({"width": 1440, "height": 900}); attendi(pg, "innerWidth === 1440"); frames(pg, 3)
                pg.click("#calcx")
                pg.click("#nav [data-v=data]"); frames(pg, 3)
        step("320 px: no horizontal scroll of the page", narrow)
        def closes():
            pg.click("#nav [data-v=draw]"); frames(pg, 3)
            pg.click("#np-calc2"); attendi(pg, "!document.querySelector('#calcdlg').hidden")
            assert not pg.evaluate("document.querySelector('#calcdlg').hidden")
            pg.fill("#calcin", ""); pg.keyboard.press("Escape"); attendi(pg, "document.querySelector('#calcdlg').hidden")
            assert pg.evaluate("document.querySelector('#calcdlg').hidden")
            pg.click("#nav [data-v=data]"); frames(pg, 3)
            assert pg.locator("#np-iso").count() == 0, "the Isotopi button is gone from the header"
        step("Esc closes the calculator; no Isotopi button in the header", closes)
        def xic_dlg():
            pg.evaluate("openXic(null,{})"); pg.wait_for_selector("#xic-mz", state="visible"); pg.fill("#xic-mz", "c14h13f4n3o2s"); attendi(pg, "document.querySelector('#xic-sum').innerText.includes('interpretata come')")
            assert "interpretata come" in pg.inner_text("#xic-sum"), pg.inner_text("#xic-sum"); pg.click("#xic-no")
        step("XIC window too", xic_dlg)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
