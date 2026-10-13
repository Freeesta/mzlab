import sys; import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
OPENPAR = "(()=>{const c=document.querySelector('.pnl.xic .cpop');if(c&&c.hidden)document.querySelector('.pnl.xic [data-a=cpar]').click()})()"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8815, wd="/tmp/wd5")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        def addion(t, ok=True):                       # t: neutral formula (the adduct comes from the selector)
            pg.evaluate("openXic(E.panels.find(p=>p.type==='xic')||null)")
            pg.fill("#xic-mz", t); hold(pg, 1300)
            if ok: pg.click("#xic-go"); ready(pg)
        def addwin(v):                                # write ONE m/z value: the unit window [n-0.2, n+0.8] is built around its nominal mass
            pg.evaluate("openXic(E.panels.find(p=>p.type==='xic')||null)")
            pg.fill("#xic-mz", v)
            pg.click("#xic-go"); ready(pg)
        addion("C14H13F4N3O2S")
        xi = pg.evaluate("E.panels.findIndex(p=>p.type==='xic')")
        sel = lambda o: f'.pnl.xic [data-o="{o}"]'
        def CORR(v):      # the "Correzione" drop-down: open it, click the choice
            pg.click('.pnl.xic [data-o="%s"]' % ("bkg" if v in ("snip", "tract", "") else "corr")); pg.wait_for_timeout(150); pg.click(f'.pnl.xic .corri[data-c="{v}"]')      # «Sottrai» (file) and «Fondo» are two menus
        def formula_in_xic():
            t = pg.evaluate(f"E.panels[{xi}].traces.map(t=>[t.mz,t.w,t.label])"); print("traces:", t)
            # the neutral formula gives the calculated m/z of the adduct (364.0737 for [M+H]+) and a window of +-0.5 Da around it
            assert t and abs(t[0][0] - 364.3) < 1e-6 and t[0][1] == 0.5 and "C14H13F4N3O2S" in t[0][2] and "m/z 364.3" in t[0][2], t
            addwin("163.3")                                       # nominal 163: window 162.8-163.8
            tr = pg.evaluate(f"E.panels[{xi}].traces[1]"); assert abs(tr["mz"] - 163.3) < 1e-6 and abs(tr["w"] - 0.5) < 1e-6, tr
            addion("C2H6Qq", ok=False)
            assert "non valida" in pg.inner_text("#xic-sum"); pg.click("#xic-no")
            # one decimal only: the value is rounded half up to 0.1, commas are accepted, and the window comes from the nominal mass (100.6 -> n = 101 -> 100.8-101.8)
            pg.evaluate("openXic(E.panels.find(p=>p.type==='xic'))"); pg.fill("#xic-mz", "100,26"); assert "99.8 - 100.8" in pg.inner_text("#xic-sum"), pg.inner_text("#xic-sum")
            pg.fill("#xic-mz", "100.6"); assert "100.8 - 101.8" in pg.inner_text("#xic-sum"), pg.inner_text("#xic-sum")
            pg.click("#xic-go"); ready(pg)
            tr = pg.evaluate(f"E.panels[{xi}].traces[2]"); assert abs(tr["mz"] - 101.3) < 1e-6 and abs(tr["w"] - 0.5) < 1e-6, tr
            pg.evaluate(f"E.panels[{xi}].traces.pop()"); pg.evaluate(f"E.panels[{xi}].traces.pop()")
        step("XIC window: formula (neutral) and one m/z value (unit window around the nominal mass)", formula_in_xic)
        def xicdlg_layout():
            pg.evaluate("openXic(E.panels.find(p=>p.type==='xic'))")
            t = pg.inner_text("#xicdlg"); assert "oppure" in t and "neutra" in t and "Extracted Ion Chromatogram" in t and "1 Da" in t, t
            assert not pg.query_selector("#xic-lo") and not pg.query_selector("#xic-hi") and "compromesso" not in t and "0.7 Da" not in t, t
            pg.fill("#xic-mz", ""); hold(pg, 700)      # (the window opens with the last ion: clear it, then nothing is written)
            pg.click("#xic-go"); err = pg.inner_text("#xic-err"); assert "Scrivi un valore di m/z" in err, err      # nothing written
            pg.fill("#xic-mz", "C2H6O"); hold(pg, 1300); assert "46.8 - 47.8" in pg.inner_text("#xic-sum"), pg.inner_text("#xic-sum")      # one box: an m/z or a formula
            pg.click("#xic-no")
        step("XIC window layout: new text, one m/z value, oppure, neutral formula; no from/to fields", xicdlg_layout)
        def blank():
            pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
            pg.evaluate(f"(()=>{{const p=E.panels[{xi}]; p.traces=p.traces.slice(0,1); p.fk=''}})()"); pg.wait_for_timeout(100)
            base = pg.evaluate(f"seriesOf(E.panels[{xi}]).then(a=>a.map(s=>[s.name,Math.max(...s.y)]))"); print("before:", base)
            pg.evaluate("E.files[2].type='blank'"); pg.evaluate(f"ctl(E.panels[{xi}])"); pg.wait_for_timeout(300)
            pg.evaluate(OPENPAR); CORR("f" + str(pg.evaluate("E.files[2].k")))   # third file = t60 marked as the 'blank'
            pg.wait_for_timeout(1500)
            after = pg.evaluate(f"seriesOf(E.panels[{xi}]).then(a=>a.map(s=>[s.name,Math.max(...s.y),s.corr]))"); print("after:", after)
            assert len(after) == len(base) - 1 and all(a[2] == "- bianco" for a in after)
            pg.screenshot(path=SH + "50_blank.png")
            pg.evaluate(OPENPAR); CORR("snip"); pg.wait_for_timeout(1200)
            sn = pg.evaluate(f"seriesOf(E.panels[{xi}]).then(a=>a.map(s=>[s.corr,Math.max(...s.y)]))"); print("snip:", sn)
            assert all("baseline" in x[0] for x in sn)
            pg.screenshot(path=SH + "51_snip.png")
        step("blank subtraction + SNIP", blank)
        def snip_unit():
            v = pg.evaluate("(()=>{const y=Array.from({length:200},(_, i)=>100+50*Math.exp(-Math.pow((i-100)/4,2)/2)); const b=FONDO.snipBaseline(y,20); return [Math.max(...b), Math.min(...b), Math.max(...y)]})()")
            print("snip unit:", v); assert 95 < v[1] and v[0] < 112 and v[2] > 140
        step("SNIP keeps the peak out of the baseline", snip_unit)
        def spec_bg():
            pg.evaluate("setView('data')")
            si = pg.evaluate("E.panels.findIndex(p=>p.type==='spec')")
            pg.evaluate(f"(()=>{{const p=E.panels[{si}]; p.r0=14.0; p.r1=14.6; p.k=0; p.bg=''; draw(p)}})()"); pg.wait_for_timeout(800)
            sm = lambda: pg.evaluate(f"E.panels[{si}]._a.data[0].d.y.reduce((a,b)=>a+b,0)")
            raw = sm()
            pg.evaluate(f"(()=>{{const p=E.panels[{si}]; p.bg='w'; p.bw0=13.0; p.bw1=14.2; ctl(p); draw(p)}})()"); pg.wait_for_timeout(1000)
            sub = sm(); print("sum raw/sub:", raw, sub, pg.evaluate(f"[E.browse,E.cur,E.panels[{si}].k,E.panels[{si}].bg,E.panels[{si}].p_all,E.panels[{si}]._a.data.length, E.panels[{si}]._a.data[0].f.k]"))
            assert "fondo sottratto" in pg.inner_text(f".pnl.spec .leg") and sub < raw
            pg.screenshot(path=SH + "52_specbg.png")
        step("spectrum background subtraction", spec_bg)
        def calc():
            pg.evaluate("document.querySelector('#np-calc2').click()"); pg.fill("#calcin", "C14H13F4N3O2S"); pg.wait_for_timeout(800)
            t = pg.inner_text("#calcout"); print(t.replace("\n", " | ")[:300]); assert "364.07" in t and "363.0665" in t and "1 decimale" not in t
            pg.screenshot(path=SH + "53_calc.png")
            pg.locator("#calcout button[data-m]").first.click(); pg.wait_for_timeout(1500)          # the XIC button opens THE XIC window, formula and window already filled in
            assert pg.evaluate("document.querySelector('#xicdlg').open") and pg.input_value("#xic-mz") == "C14H13F4N3O2S" and "363.8 - 364.8" in pg.inner_text("#xic-sum"), (pg.input_value("#xic-mz"), pg.inner_text("#xic-sum"))
            n0 = pg.evaluate("E.panels.filter(p=>p.type==='xic').reduce((a,p)=>a+p.traces.length,0)"); pg.click("#xic-go"); ready(pg)
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').reduce((a,p)=>a+p.traces.length,0)") == n0 + 1
        step("calculator", calc)
        def draw_ion():
            pg.evaluate("setView('draw')"); pg.wait_for_function("TPDraw.ready()", timeout=60000); pg.wait_for_timeout(1500)
            pg.fill("#ex-smi", "Oc1ccc(cc1)C(=O)N"); pg.click("#ex-load"); pg.wait_for_timeout(2500)
            # the boxes that did the student's work (caption, formula/adduct table, fragment selection) no longer exist
            assert pg.evaluate("['selcard','selbody','dcard','dcomp','cap-name','cap-part','cap-ad','cap-on','cap-txt','cap-copy'].every(i=>!document.getElementById(i))")
            LB = "[...document.getElementById('kframe').contentWindow.ketcher.editor.render.paper.canvas.querySelectorAll('#qqq-labels text')].map(t=>t.textContent)"
            assert pg.evaluate(LB) == ["C7H7NO2", "nominal mass 137"], pg.evaluate(LB)
            pg.select_option("#lb-dec", "4"); pg.wait_for_timeout(300)
            assert pg.evaluate(LB) == ["C7H7NO2", "exact mass 137.0477"], pg.evaluate(LB)
            pg.select_option("#lb-dec", "0"); pg.wait_for_timeout(300)
            pg.fill("#ex-smi", "Oc1ccc(cc1)C(=O)[NH3+]"); pg.click("#ex-load"); pg.wait_for_timeout(2500)         # a structure drawn with its charge: formula of the ion and m/z
            pg.evaluate("(()=>{const k=document.getElementById('kframe').contentWindow.ketcher;k.setMolecule('Oc1ccc(cc1)C(=O)[NH3+]')})()"); pg.wait_for_timeout(1500)
            t = pg.evaluate(LB); print("ION:", t); assert t == ["C7H8NO2+", "m/z 138"], t
            pg.screenshot(path=SH + "54_ion.png")
            with pg.expect_download() as d: pg.click("#ex-svg")
            svg = open(d.value.path(), encoding="utf-8").read()
            assert "m/z" in svg and "138" in svg and 'font-style="italic"' in svg and "<tspan" in svg and "TP1" not in svg, svg[-500:]
            with pg.expect_download() as d: pg.click("#ex-png")
            import os; sz = os.path.getsize(d.value.path()); print("png bytes", sz); assert sz > 5000
            open(SH + "55_ion.png", "wb").write(open(d.value.path(), "rb").read())
        step("drawing: no helper boxes, drawn charge gives the ion label (canvas, svg, png)", draw_ion)
        pg.wait_for_timeout(500)
finally:
    r.close()
for s in steps: print(s)
r.report()
