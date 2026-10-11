"""Window 'Da dove viene questo ione?': entry points, evidence sections, table click -> overlay, hypothesis saved, xlsx, no verdict words."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_origine.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8841, wd="/tmp/wd_orig")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in ["B_FullMass-t0", "B_FullMass-t10", "B_FullMass-t15", "B_FullMass-t30 (2)", "B_FullMass-t60"]]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(600)
        def entry():
            pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:194.3,w:0.5,label:'m/z 194'}]})"); pg.wait_for_timeout(1500)
            assert pg.locator("[data-o=orig]").count() == 0, "the button is hidden (decision of 7/10: the window stays in the code, no entry point)"
            pg.evaluate("openOrigin({mz:194.3,k:E.files.find(f=>f.kind==='full').k,rt:14.3})"); pg.wait_for_timeout(500)
            assert pg.evaluate("document.getElementById('ogdlg').open") and pg.input_value("#og-mz") == "194.3"
        step("no entry point in the interface; the window still opens from code with the ion filled", entry)
        def run():
            pg.fill("#og-par", "364.4"); pg.click("#og-go"); pg.wait_for_selector("#og-tb", timeout=60000); pg.wait_for_timeout(1500)
            assert pg.locator("#og-out section:visible").count() == 5
            txt = pg.inner_text("#og-out")
            for w in ["Pearson", "F contro P", "cinetica", "La tua ipotesi"]: assert w.lower() in txt.lower(), w
            for bad in ["è un frammento in sorgente.", "verdetto:", "conclusione:"]: assert bad not in txt
            assert pg.locator("#og-tb tr[data-m]").count() >= 1
            pg.screenshot(path=SH + "orig_a.png", full_page=True)
        step("evidence computed on real files: five sections, table, no verdict", run)
        def overlay():
            n0 = pg.locator("#og-chips span").count()
            pg.locator("#og-tb tr[data-m]").first.click(); pg.wait_for_timeout(1500)
            assert pg.locator("#og-chips span").count() == n0 + 1 and pg.locator("#og-tb tr.sel").count() == 1
        step("clicking a table row adds that ion to the overlaid XICs", overlay)
        def hyp():
            pg.fill("#og-hyp", "Ipotesi di prova"); pg.wait_for_timeout(900)
            assert "Ipotesi di prova" in pg.evaluate("JSON.stringify(NB.origin)")
        step("the hypothesis goes in the notebook", hyp)
        def xl():
            with pg.expect_download() as d: pg.click("#og-xlsx")
            f = d.value.path(); sh = xlsx_sheets(f); assert "Riassunto" in sh and "F contro P" in sh and "Cinetica" in sh, sh
            rows = xlsx_rows(f); flat = [c[1] for r_ in rows for c in r_ if c]
            assert "Ipotesi di prova" in flat
        step("xlsx export with the hypothesis", xl)
        def menus():
            pg.evaluate("document.getElementById('ogdlg').close()")
            pg.evaluate("addPanel('map',{k:E.files.find(f=>f.kind==='full').k})"); pg.wait_for_timeout(2500)
            pos = pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map');q.el.scrollIntoView({block:'center'});const c=q.cv.getBoundingClientRect();return [c.left+c.width*0.4,c.top+c.height*0.5]})()")
            pg.mouse.click(pos[0], pos[1], button="right"); pg.wait_for_timeout(400)
            assert pg.locator("#ctx div", has_text="Da dove viene").count() == 0
            pg.keyboard.press("Escape")
        step("no «Da dove viene» in the right-click menu of the map", menus)
        def teoria():
            pg.evaluate("document.getElementById('ogdlg').close()")
            pg.evaluate("setView('teoria')") if False else None
            h = pg.evaluate("fetch('static/teoria/17-origine.html').then(r=>r.text())"); assert "atrazina" in h and "flufenacet" not in h.lower()
            assert "Capitolo 17" in h
        step("Teoria chapter 17 exists and does not discuss the lab compound", teoria)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
