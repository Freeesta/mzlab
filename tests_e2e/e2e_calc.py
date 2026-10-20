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
            pg.click("#np-calc2"); pg.wait_for_timeout(300)
            assert pg.evaluate("window.BARRA.curTab === 'calc' && !document.querySelector('#sb-panel-calc').hidden")
            # In Disegno view, np-calc2 opens floating dropdown right under its button, not modal, >= 520px wide
            pg.click("#nav [data-v=draw]"); pg.wait_for_timeout(400)
            pg.click("#np-calc2"); pg.wait_for_timeout(300)
            b = pg.evaluate("(()=>{const a=document.querySelector('#np-calc2').getBoundingClientRect(),c=document.querySelector('#calcdlg');const r=c.getBoundingClientRect();return {ab:a.bottom,ct:r.top,w:r.width,vis:!c.hidden,modal:c.matches(':modal')}})()")
            assert b["vis"] and not b["modal"] and 0 < b["ct"] - b["ab"] < 20 and b["w"] >= 520, b
            pg.click("#calcx"); pg.wait_for_timeout(200)
            pg.click("#nav [data-v=data]"); pg.wait_for_timeout(400)
            if not pg.evaluate("!!document.querySelector('#calcin').offsetParent"):    # the sidebar tab stays open across views: the button is a toggle
                pg.click("#np-calc2"); pg.wait_for_timeout(300)
        step("in Dati opens sidebar tab, in Disegno opens right under its button >= 520 px wide", open_below)
        def usable():
            pg.wait_for_timeout(300)               # docked next to the charts the calculator stays open while the student works on them
            c = pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='chrom'&&q.tab==='full'),r=p.cv.getBoundingClientRect();return {x:r.left+p._a.X(14.3),y:r.bottom-60}})()")
            pg.evaluate("window.scrollTo(0,0)")
            pg.mouse.click(c["x"], c["y"] if c["y"] < 700 else 400); pg.wait_for_timeout(500)
            assert pg.evaluate("document.querySelector('#calcdlg').hidden") is False, "stays open while the student works on the graphs"
            assert pg.evaluate("E.panels.find(q=>q.type==='chrom'&&q.tab==='full').cur") is not None
        step("the graphs stay usable with the calculator open", usable)
        def lower():
            pg.fill("#calcin", "c14h13f4n3o2s"); until(pg, "document.querySelector('#calcsum').innerText.includes('interpretata come')")
            assert "interpretata come" in pg.inner_text("#calcsum") and "C14H13F4N3O2S" in pg.inner_text("#calcsum").replace(" ", "")
            t = pg.inner_text("#calcout"); assert "364.07" in t and "1 decimale" not in t and "spostato di qualche decimo" not in t, t
            pg.fill("#calcin", "C9h10Cl2n2O"); until(pg, SUM + ".includes('C9H10Cl2N2O')"); assert "C9H10Cl2N2O" in pg.inner_text("#calcsum").replace(" ", "")
            pg.fill("#calcin", "co"); until(pg, SUM + ".includes('CO')"); assert "CO" in pg.inner_text("#calcsum"), pg.inner_text("#calcsum")
        step("lower-case formula: interpreted line, no extra column / sentence", lower)
        def arithmetic():
            pg.click("#np-calc2") if pg.evaluate("document.querySelector('#calcdlg').hidden") else None
            pg.fill("#calcin", "364.4-194.2"); until(pg, "document.querySelector('#calcres').innerText.trim() === '170.2'")
            assert pg.inner_text("#calcres").strip() == "170.2" and not pg.evaluate("document.querySelector('#calcpad').hidden")
            assert pg.is_visible("#calc-nl") and "170.2" in pg.get_attribute("#calc-nl", "title")
            pg.fill("#calcin", "364,4 \u2212 194,2"); pg.wait_for_timeout(200); assert pg.inner_text("#calcres").strip() == "170.2", pg.inner_text("#calcres")
            pg.fill("#calcin", "(229.1-171.2)*2"); until(pg, "document.querySelector('#calcres').innerText.trim() === '115.8'"); assert pg.inner_text("#calcres").strip() == "115.8"
            pg.click("#calc-nl"); until(pg, "window.BARRA.curTab === 'losses' && !document.querySelector('#sb-panel-losses').hidden")
            assert pg.evaluate("window.BARRA.curTab === 'losses' && !document.querySelector('#sb-panel-losses').hidden")
            assert pg.input_value("#nl-q") == "115.8"
            pg.click("#np-calc2"); pg.wait_for_timeout(200)
        step("a calculation: 364.4-194.2 = 170.2 (also with comma and the minus sign), calc-nl button opens losses tab", arithmetic)
        def keypad():
            pg.fill("#calcin", ""); pg.wait_for_timeout(100)
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
            pg.fill("#calcin", "C9H10Cl2N2O"); until(pg, "document.querySelector('#calcpad').hidden")
            assert pg.evaluate("document.querySelector('#calcpad').hidden") and "C9H10Cl2N2O" in pg.inner_text("#calcout").replace(" ", "") and pg.locator("#calcout table").count() == 1
            assert pg.evaluate("document.querySelector('#calcres').hidden")
            pg.fill("#calcin", ""); until(pg, "!document.querySelector('#calcpad').hidden")
        step("a formula: adduct table as before, no keypad", formula_hides_pad)
        def closes():
            pg.click("#nav [data-v=draw]"); pg.wait_for_timeout(300)
            pg.click("#np-calc2"); pg.wait_for_timeout(300)
            assert not pg.evaluate("document.querySelector('#calcdlg').hidden")
            pg.fill("#calcin", ""); pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
            assert pg.evaluate("document.querySelector('#calcdlg').hidden")
            pg.click("#nav [data-v=data]"); pg.wait_for_timeout(300)
            assert pg.locator("#np-iso").count() == 0, "the Isotopi button is gone from the header"
        step("Esc closes the calculator; no Isotopi button in the header", closes)
        def xic_dlg():
            pg.evaluate("openXic(null,{})"); pg.wait_for_timeout(300); pg.fill("#xic-mz", "c14h13f4n3o2s"); hold(pg, 1200)
            assert "interpretata come" in pg.inner_text("#xic-sum"), pg.inner_text("#xic-sum"); pg.click("#xic-no")
        step("XIC window too", xic_dlg)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
