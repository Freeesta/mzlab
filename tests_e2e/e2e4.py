import sys; import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
def pt(pg, i, x=None, y=0.5, ymz=None):
    return pg.evaluate("""([i,x,fy,mz])=>{const p=E.panels[i],r=p.cv.getBoundingClientRect(),a=p._a;
      return {px:r.left+(x==null?(M.l+(a.W-M.r))/2:a.X(x)), py:r.top+(mz!=null?a.Y(mz):(r.height-60)*fy+10)}}""", [i, x, y, ymz])
r = Run(motore="python")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
        pg.screenshot(path=SH + "40_start.png")
        # --- hover tooltip on the chromatogram
        def hover_chrom():
            c = pt(pg, 0, 14.33); pg.mouse.move(c["px"] - 30, c["py"]); pg.mouse.move(c["px"], c["py"], steps=4); pg.wait_for_timeout(300)
            t = pg.inner_text(".pnl.chrom .tip"); print("TIP:", t.replace("\n", " | "))
            assert "RT 14.3" in t and "B_FullMass" in t
            vis = pg.evaluate("[...document.querySelectorAll('.vl')].filter(v=>!v.hidden).length"); print("visible vlines:", vis); assert vis >= 1
            pg.screenshot(path=SH + "41_hover.png")
        step("hover tooltip + synced line", hover_chrom)
        # --- peaks, stack, log, legend toggle on chromatogram
        def nopeaks():
            assert pg.locator('[data-o="peaks"]').count() == 0
        step("no peak-label option anywhere", nopeaks)
        def stack():
            pg.locator('.pnl.chrom [data-o="mode"]').select_option("stk"); pg.wait_for_timeout(800); pg.screenshot(path=SH + "43_stack.png")
            assert pg.evaluate("E.panels[0]._a.stk === true")
        step("stacked view", stack)
        def stack_int():
            # right click in the 2nd band -> integrate -> should integrate that series
            a = pg.evaluate("(()=>{const p=E.panels[0],a=p._a;return {n:a.sr.length}})()")
            c = pt(pg, 0, 14.33, ymz=1.45); pg.mouse.click(c["px"], c["py"], button="right"); pg.wait_for_timeout(300)
            pg.locator("#ctx div", has_text="Integra il picco").first.click(); pg.wait_for_timeout(500)
            assert pg.is_visible("#askdlg") and "non si integra" in pg.inner_text("#asktxt"), "TIC must refuse the integration"
            pg.click("#askno"); assert pg.evaluate("E.panels[0].ints.length") == 0
        step("TIC refuses integration (only XIC)", stack_int)
        def logv():
            pg.locator('.pnl.chrom [data-o="mode"]').select_option("ovl"); pg.click('.pnl.chrom [data-a="log"]'); pg.wait_for_timeout(800)
            pg.screenshot(path=SH + "44_log.png"); assert pg.evaluate("E.panels[0]._a.logy === true")
            pg.click('.pnl.chrom [data-a="log"]'); pg.wait_for_timeout(300)
        step("log scale", logv)
        def legtoggle():
            n0 = pg.evaluate("E.panels[0]._a.sr.length")   # full-scan and MS2 files (MRM files have their own panel)
            pg.locator('.pnl.chrom .leg [data-h]').first.click(); pg.wait_for_timeout(600)
            n = pg.evaluate("E.panels[0]._a.sr.length"); assert n == n0 - 1, (n0, n)
            pg.screenshot(path=SH + "45_legend_hidden.png")
            pg.locator('.pnl.chrom .leg [data-h]').first.click(); pg.wait_for_timeout(500)
            assert pg.evaluate("E.panels[0]._a.sr.length") == n0
        step("legend toggle", legtoggle)
        def wheelpan():
            c = pt(pg, 0, 14.3); pg.mouse.move(c["px"], c["py"])
            pg.keyboard.down("Control")
            for _ in range(4): pg.mouse.wheel(0, -100); pg.wait_for_timeout(150)
            pg.keyboard.up("Control"); pg.wait_for_timeout(500)
            z = pg.evaluate("E.panels[0].zoom"); print("zoom after wheel:", z); assert z and z[1] - z[0] < 12, z
            z0 = z[0]
            pg.keyboard.down("Shift"); pg.mouse.down(); pg.mouse.move(c["px"] + 80, c["py"], steps=6); pg.mouse.up(); pg.keyboard.up("Shift"); pg.wait_for_timeout(500)
            z2 = pg.evaluate("E.panels[0].zoom"); print("zoom after pan:", z2); assert z2[0] < z0 - 0.01
            pg.screenshot(path=SH + "46_zoom_pan.png")
            pg.click("#dpanels .pnl >> nth=0 >> [data-a=fit]"); pg.wait_for_timeout(500); assert pg.evaluate("E.panels[0].zoom") is None
        step("ctrl+wheel zoom, shift pan, fit button reset", wheelpan)
        # --- ion map
        def mapadd():
            pg.click("#np-map"); ready(pg)
            i = pg.evaluate("E.panels.findIndex(p=>p.type==='map')"); assert i >= 0 and pg.evaluate(f"E.panels[{i}]._a !== null")
            pg.screenshot(path=SH + "47_map.png")
        step("add ion map", mapadd)
        mi = lambda: pg.evaluate("E.panels.findIndex(p=>p.type==='map')")
        def maphover():
            i = mi(); pg.evaluate(f"E.panels[{i}].el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            c = pt(pg, i, 14.33, ymz=194.5); pg.mouse.move(c["px"] - 20, c["py"]); pg.mouse.move(c["px"], c["py"], steps=4); pg.wait_for_timeout(300)
            t = pg.inner_text(f".pnl.map .tip"); print("MAP TIP:", t.replace("\n", " | ")); import re; mzv = float(re.search(r"m/z ([0-9.]+)", t).group(1)); assert abs(mzv - 194.5) < 3.5, t   # +-3.5 m/z = about 2 px: the page layout may shift by a sub-pixel
        step("map hover", maphover)
        def mapxic():
            i = mi(); c = pt(pg, i, 14.33, ymz=194.5); pg.keyboard.down("Shift"); pg.mouse.click(c["px"], c["py"], button="right"); pg.keyboard.up("Shift"); pg.wait_for_timeout(300)   # Maiusc + clic destro = XIC (mappa.js; il clic destro blocca il punto)
            pg.click("#xic-go"); ready(pg)
            assert pg.evaluate("E.panels.some(p=>p.type==='xic'&&p.traces.length===1&&Math.abs(p.traces[0].mz-194.5)<3.5)")
        step("map Shift + right click -> XIC", mapxic)
        def mapdrag():
            i = mi(); a = pt(pg, i, 14.1, ymz=300); b = pt(pg, i, 14.6, ymz=300)
            pg.mouse.move(a["px"], a["py"]); pg.keyboard.down("Alt"); pg.mouse.down(); pg.mouse.move(b["px"], b["py"], steps=6); pg.mouse.up(); pg.keyboard.up("Alt"); pg.wait_for_timeout(300)   # Alt + trascina = intervallo (trascinare da solo = zoom)
            assert pg.evaluate(f"E.panels[{i}].sel"), "Alt + drag selects an interval"
            pg.click(".pnl.map [data-mz=menu]"); pg.wait_for_timeout(300)                                  # the «⋯» menu: with a stretch selected the Spectrum entry is the average
            pg.locator("#ctx div", has_text="Spettro medio").first.click(); pg.wait_for_timeout(2000)
            assert pg.evaluate("E.panels.filter(p=>p.type==='spec').length") >= 2
        step("map Alt + drag, menu ⋯ -> averaged spectrum", mapdrag)
        def mapdiff():
            i = mi(); pg.evaluate(f"E.panels[{i}].ref = 0; E.panels[{i}].k = 2; ctl(E.panels[{i}]); draw(E.panels[{i}])"); pg.wait_for_timeout(2000)
            pg.screenshot(path=SH + "48_map_diff.png")
            assert pg.evaluate(f"E.panels[{i}]._img.b !== null")
        step("map difference (t60 - t0)", mapdiff)
        def mapzoom():
            i = mi(); c = pt(pg, i, 14.33, ymz=194.5); pg.mouse.move(c["px"], c["py"])
            pg.keyboard.down("Control"); pg.mouse.wheel(0, -400); pg.keyboard.up("Control"); pg.wait_for_timeout(600)
            z = pg.evaluate(f"[E.panels[{i}].zoom, E.panels[{i}].zoomY]"); print("map zoom", z); assert z[0] and z[1]
            pg.screenshot(path=SH + "49_map_zoom.png")
        step("map ctrl+wheel zoom (both axes)", mapzoom)
        # --- persistence
        pg.wait_for_timeout(1500)
        pg.reload(); pg.wait_for_timeout(5000)
        def restored():
            print(pg.evaluate("E.panels.map(p=>p.type+':'+(p.mode||'')+':'+(p.peaks?'P':'')+':'+(p.scale||'')+':'+(p.ref===''?'':p.ref))"))
            assert pg.evaluate("E.panels.some(p=>p.type==='map')")
            pg.screenshot(path=SH + "50_restored.png")
        step("session restore of new panel options", restored)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
