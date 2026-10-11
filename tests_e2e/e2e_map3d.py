"""3D view of the RT-m/z map, 2D/3D switch, difference between two full scans with normalisation."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_map3d.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8831, wd="/tmp/wd_m3d")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in ["B_FullMass-t0", "B_FullMass-t15", "B_FullMass-t60"]]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(600)
        def add():
            pg.click("#np-map"); ready(pg)
            assert pg.evaluate("E.panels.some(q=>q.type==='map')")
        step("map panel opens (2D)", add)
        mid = lambda: pg.evaluate("E.panels.find(q=>q.type==='map').id")
        def three():
            pg.evaluate("id=>{const q=E.panels.find(x=>x.id===id);q.el.scrollIntoView({block:'center'});}", mid())
            pg.locator("[data-o=view][data-v='3d']").first.click(); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.panels.find(q=>q.type==='map')._a.is3d") is True
            px = pg.evaluate("(()=>{const c=E.panels.find(q=>q.type==='map').cv.getBoundingClientRect();return [c.left+c.width/2,c.top+c.height/2]})()")
            pg.screenshot(path=SH + "m3d_0.png", full_page=True)
            n0 = pg.evaluate("E.panels.find(q=>q.type==='map').az")
            pg.mouse.move(px[0], px[1]); pg.mouse.down(); pg.mouse.move(px[0] + 80, px[1] + 30, steps=6); pg.mouse.up(); pg.wait_for_timeout(1200)
            assert pg.evaluate("E.panels.find(q=>q.type==='map').az") != n0
            pg.screenshot(path=SH + "m3d_a.png", full_page=True)
        step("3D view draws and rotates by dragging", three)
        def diff():
            pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map');q.ref=E.files.filter(f=>f.kind==='full')[1].k;q.norm='max';ctl(q);draw(q)})()"); pg.wait_for_timeout(2500)
            assert "normalizzata" in pg.evaluate("E.panels.find(q=>q.type==='map').leg.textContent")
            pg.screenshot(path=SH + "m3d_b.png", full_page=True)
        step("difference between two files, normalised, in 3D", diff)
        def back():
            pg.locator("[data-v3=top]").first.click(); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.panels.find(q=>q.type==='map')._a.is3d") is not True
            pg.screenshot(path=SH + "m3d_c.png", full_page=True)
        step("back to 2D keeps the difference", back)
        def saved():
            pg.evaluate("E.panels.find(q=>q.type==='map').view='3d';uiSave()"); hold(pg, 800)
            assert '"view":"3d"' in pg.evaluate("JSON.stringify(NB.ui.panels||NB.ui)")
        step("view saved in the notebook", saved)
        def surf():
            pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map');q.ref='';q.norm='abs';q.view='3d';q.az=25;q.elv=38;q._pin3=null;ctl(q);draw(q)})()"); pg.wait_for_timeout(2500)
            g = pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map');return [q._g3.nx,q._g3.ny,q._g3.zmax]})()")
            assert g[0] <= 300 and g[1] <= 300 and g[2] > 0, g                                   # at most 300 x 300 cells
            assert pg.locator("[data-v3=pal]").count() == 1 and pg.locator("[data-v3=sig]").count() == 1 and pg.locator("[data-v3=thr]").count() == 1
            pg.screenshot(path=SH + "m3d_surf.png", full_page=True)
            k0 = pg.evaluate("E.panels.find(q=>q.type==='map')._g3.key")
            pg.fill("[data-v3=sig]", "2"); pg.dispatch_event("[data-v3=sig]", "change"); pg.wait_for_timeout(1200)
            assert pg.evaluate("E.panels.find(q=>q.type==='map')._g3.key") != k0                  # the smoothing is applied
            pg.select_option("[data-v3=pal]", "cividis"); pg.wait_for_timeout(800)
            assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.mappa')).t3pal") == "cividis"
            pg.screenshot(path=SH + "m3d_cividis.png", full_page=True)
            ms = pg.evaluate("(async()=>{const q=E.panels.find(x=>x.type==='map');q._rot=true;const t=performance.now();for(let i=0;i<10;i++){q.az=25+i*3;await drawMap(q)}q._rot=false;return (performance.now()-t)/10})()")
            print("turning: ms per frame", ms); assert ms < 60, ms                                 # about 30 fps while turning (lenient on a slow CI)
        step("surface: grid up to 300x300, Gaussian smoothing, palette cividis, speed while turning", surf)
        def views():
            pg.locator("[data-v3=rt]").click(); pg.wait_for_timeout(800)
            assert pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map');return [q.az,q.elv]})()") == [0, 0]
            pg.locator("[data-v3=mz]").click(); pg.wait_for_timeout(800)
            assert pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map');return [q.az,q.elv]})()") == [90, 0]
            pg.screenshot(path=SH + "m3d_side.png", full_page=True)
            pg.locator("[data-v3=iso]").click(); pg.wait_for_timeout(800)
        step("preset views: side along m/z = chromatogram, side along RT = spectrum", views)
        def pin():
            c = pg.evaluate("(()=>{const q=E.panels.find(x=>x.type==='map'),c=q._a.centres.filter(t=>t.v>0.3).sort((a,b)=>b.v-a.v)[0],r=q.cv.getBoundingClientRect();return [r.left+c.x,r.top+c.y]})()")
            pg.mouse.click(c[0], c[1]); pg.wait_for_timeout(800)
            assert pg.evaluate("!!E.panels.find(q=>q.type==='map')._pin3")                           # the line with RT and m/z
            n0 = pg.evaluate("E.panels.filter(q=>q.type==='spec').length")
            pg.mouse.dblclick(c[0], c[1]); pg.wait_for_timeout(2500)
            assert pg.evaluate("E.panels.filter(q=>q.type==='spec').length") > n0                   # double click = spectrum
            pg.screenshot(path=SH + "m3d_pin.png", full_page=True)
        step("click = line with RT and m/z; double click = spectrum", pin)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
