import sys, json; import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
def pt(pg, idx, x=None, y=None):
    """page coords of canvas point in panel idx at data x (and fractional y 0..1 of plot height)."""
    return pg.evaluate("""([i,x,fy])=>{const p=E.panels[i],r=p.cv.getBoundingClientRect(),a=p._a;
        return {px:r.left+(x==null?(M.l+(a.W-M.r))/2:a.X(x)), py:r.top+(fy==null?r.height/2:(r.height-60)*fy+10)}}""", [idx, x, y])
def click_menu(pg, text):
    pg.locator("#ctx div", has_text=text).first.click()
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:200]))
r = Run()
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES])
        pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.screenshot(path=SH + "10_opened.png")
        # find the TIC apex of the first full-scan file
        info = pg.evaluate("""()=>{const p=E.panels[0];const s=p._a.sr[0];let m=0;s.ys.forEach((v,i)=>{if(v>s.ys[m])m=i});return {rt:s.x[m],n:p._a.sr.length}}""")
        print("TIC apex", info)
        rt = info["rt"]
        def drag_spec():
            a = pt(pg, 0, rt - 0.15); b = pt(pg, 0, rt + 0.15)
            pg.mouse.move(a["px"], a["py"]); pg.mouse.down(); pg.mouse.move(b["px"], b["py"], steps=8); pg.mouse.up(); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels[1].r0") is not None
        step("drag chromatogram -> spectrum", drag_spec)
        pg.screenshot(path=SH + "11_spectrum.png")
        # right click on tallest spectrum peak
        def xic_from_spec():
            top = pg.evaluate("""()=>{const p=E.panels[1],d=p._a.data[0].d;let m=0;d.y.forEach((v,i)=>{if(v>d.y[m])m=i});return {mz:d.mz[m]}}""")
            print("spec top m/z", top)
            c = pt(pg, 1, top["mz"])
            pg.mouse.click(c["px"], c["py"], button="right"); pg.wait_for_timeout(300)
            click_menu(pg, "Estrai l'XIC"); pg.wait_for_timeout(2500)      # the XIC appears at once, no window
            assert pg.evaluate("E.panels.some(p=>p.type==='xic')")
        step("spectrum right-click -> XIC panel", xic_from_spec)
        pg.screenshot(path=SH + "12_xic.png")
        def add_ion():
            xi = pg.evaluate("E.panels.findIndex(p=>p.type==='xic')")
            el = pg.locator(".pnl.xic").first
            pg.evaluate("openXic(E.panels.find(p=>p.type==='xic'))"); pg.fill("#xic-mz", "200"); pg.click("#xic-go"); ready(pg)
            assert pg.evaluate(f"E.panels[{xi}].traces.length") == 2
        step("add second ion", add_ion)
        def split_merge():
            pg.locator('.pnl.xic [data-o="split"]').first.click(); pg.wait_for_timeout(2000)
            n = pg.evaluate("E.panels.filter(p=>p.type==='xic').length"); assert n == 2, n
        step("split XIC", split_merge)
        def integrate():
            xi = pg.evaluate("E.panels.findIndex(p=>p.type==='xic')")
            rt2 = pg.evaluate(f"""()=>{{const s=E.panels[{xi}]._a.sr[0];let m=0;s.ys.forEach((v,i)=>{{if(v>s.ys[m])m=i}});return s.x[m]}}""")
            c = pt(pg, xi, rt2)
            pg.mouse.click(c["px"], c["py"], button="right"); pg.wait_for_timeout(300)
            click_menu(pg, "Integra il picco"); pg.wait_for_timeout(500)
            n = pg.evaluate(f"E.panels[{xi}].ints.length"); assert n >= 1, n
            # drag a bar edge
            it = pg.evaluate(f"E.panels[{xi}].ints[0]")
            a = pt(pg, xi, it["a"]); b = pt(pg, xi, it["a"] - 0.1)
            pg.mouse.move(a["px"], a["py"]); pg.mouse.down(); pg.mouse.move(b["px"], b["py"], steps=6); pg.mouse.up()
            it2 = pg.evaluate(f"E.panels[{xi}].ints[0]"); assert it2["a"] < it["a"] - 0.01, (it, it2)
        step("integrate + drag bar", integrate)
        pg.screenshot(path=SH + "13_integrated.png")
        def ints_tbl():
            pg.locator('.pnl.xic [data-a="itab"]').first.click(); pg.wait_for_timeout(600)
            assert pg.locator("#bigdlg[open] table").count() == 1
            pg.screenshot(path=SH + "14_ints.png"); pg.click("#bigx")
        step("integration table", ints_tbl)
        def method():
            pg.click("#np-method"); ready(pg)
            txt = pg.inner_text("#bigdlg"); assert "Metodo" in txt or len(txt) > 50
            pg.screenshot(path=SH + "15_method_full.png"); pg.click("#bigx")
            pg.evaluate("E.cur=1;E.browse=true;renderFileList();renderNav()")   # MRM file
            pg.click("#np-method"); ready(pg)
            pg.screenshot(path=SH + "16_method_mrm.png"); pg.click("#bigx")
            pg.evaluate("E.browse=false;E.cur=0;renderFileList();renderNav();redrawAll()")
        step("method popup (full + mrm)", method)
        def nav():
            pg.evaluate("E.browse=true;renderNav();redrawAll()"); pg.wait_for_timeout(800)      # the file arrows work with "Solo il file selezionato" (with "Tutti sovrapposti" they are disabled)
            pg.click("#fnext"); ready(pg); c1 = pg.evaluate("E.cur"); pg.click("#fnext"); ready(pg)
            pg.keyboard.press("ArrowLeft"); pg.wait_for_timeout(1000)
            assert pg.evaluate("E.panels.some(p=>p.type==='xic'&&p.traces.length>=1)")
        step("file arrows keep XIC", nav)
        pg.screenshot(path=SH + "18_nav.png")
        pg.wait_for_timeout(1500)   # autosave
        # reload page: session restore
        pg.reload(); pg.wait_for_timeout(4000)
        pg.screenshot(path=SH + "20_restored.png")
        print("after reload", pg.evaluate("({f:E.files.length,p:E.panels.map(p=>p.type+':'+(p.traces||[]).length+':'+p.ints.length)})"))
        step("tabs Dati, Disegno, Teoria (no Tabui without the administrator mode)", lambda: (lambda n: None if n == ["data", "draw", "theory"] else (_ for _ in ()).throw(AssertionError(n)))(pg.evaluate("[...document.querySelectorAll('#nav button')].map(b=>b.dataset.v)")))
        pg.click("text=Disegno"); pg.wait_for_timeout(5000); pg.screenshot(path=SH + "22_draw.png")
        b = None
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
