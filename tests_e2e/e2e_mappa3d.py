"""3D viewer of the RT x m/z map (mappa3d.js, WebGL2): opened from the «Apri in 3D» button of the 2D panel on a low resolution file and on a high
resolution one: it draws, turns, changes representation, points and reads RT / m/z / intensity coherent with the 2D map, fixes a point that shows up in
the table of the 2D map, Esc closes and the focus goes back to the button."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa3d.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
M = "E.panels.find(q=>q.type==='map')"
BTN = ".pnl.map [data-o=view][data-v='3d']"
D = synth("m3d")
r = Run(port=8838, wd="/tmp/wd_mappa3d")
INK = "(()=>{const c=document.querySelector('.m3d-gl');window.__m3d.frame();const o=document.createElement('canvas');o.width=c.width;o.height=c.height;const g=o.getContext('2d');g.drawImage(c,0,0);const d=g.getImageData(0,0,o.width,o.height).data;const s=new Set();for(let i=0;i<d.length;i+=4*97)s.add((d[i]>>4)+','+(d[i+1]>>4)+','+(d[i+2]>>4));return s.size})()"


def viewer(pg, tag, hr):
    def opens():
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(300)
        assert pg.locator(BTN).count() == 1
        pg.locator(BTN).click()
        pg.wait_for_function("document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g && window.__m3d.V.Z", timeout=60000); pg.wait_for_timeout(500)
        assert pg.evaluate("document.querySelector('dialog.m3d').matches(':modal')")
        assert pg.evaluate(INK) > 6, "the canvas draws something"
        pg.screenshot(path=SH + f"m3d_{tag}_surf.png")
    step(f"{tag}: «Open in 3D» opens the modal viewer and the surface draws", opens)
    def region():
        a = pg.evaluate("[window.__m3d.V.rt, window.__m3d.V.mz]"); z = pg.evaluate(f"[{M}._a.x0,{M}._a.x1,{M}._a.y0,{M}._a.y1]")
        assert abs(a[0][0] - z[0]) < 1e-6 and abs(a[0][1] - z[1]) < 1e-6 and abs(a[1][0] - z[2]) < 1e-6 and abs(a[1][1] - z[3]) < 1e-6, (a, z)
        g = pg.evaluate("[window.__m3d.V.g.nrt, window.__m3d.V.g.nmz, window.__m3d.V.g.log]")
        assert g[0] <= 1024 and g[1] <= 1024 and g[0] > 2 and g[1] > 2 and bool(g[2]) == hr, g
    step(f"{tag}: region of the 2D map, grid up to 1024 and cells in ppm only in high resolution", region)
    def rotate():
        box = pg.locator(".m3d-gl").bounding_box(); cx, cy = box["x"] + box["width"] * 0.4, box["y"] + box["height"] * 0.5
        az0 = pg.evaluate("window.__m3d.V.cam.az"); pg.mouse.move(cx, cy); pg.mouse.down(); pg.mouse.move(cx + 120, cy + 20, steps=6); pg.mouse.up(); pg.wait_for_timeout(300)
        assert pg.evaluate("window.__m3d.V.cam.az") != az0
        d0 = pg.evaluate("window.__m3d.V.cam.d"); pg.mouse.move(cx, cy); pg.mouse.wheel(0, -300); pg.wait_for_timeout(200)
        assert pg.evaluate("window.__m3d.V.cam.d") < d0
        pg.keyboard.press("1"); pg.wait_for_timeout(200); assert pg.evaluate("window.__m3d.V.cam.el") > 80
        pg.keyboard.press("2"); pg.wait_for_timeout(200); assert pg.evaluate("window.__m3d.V.cam.el") == 0
        pg.keyboard.press("r"); pg.wait_for_timeout(1500)
    step(f"{tag}: drag rotates, wheel zooms, keys 1-2 and R", rotate)
    def modes():
        modes = ["fall", "cont", "surf"] + (["sticks"] if hr else [])
        for m in modes:
            pg.select_option("dialog.m3d [data-m=mode]", m); pg.wait_for_timeout(1500 if m == "sticks" else 600)
            assert pg.evaluate(INK) > 4, m
            pg.screenshot(path=SH + f"m3d_{tag}_{m}.png")
        dis = pg.evaluate("document.querySelector('dialog.m3d [data-m=mode] [value=sticks]').disabled")
        assert dis == (not hr), "sticks only for high resolution"
        if hr:
            n = pg.evaluate("window.__m3d.V.pts && window.__m3d.V.pts.n"); assert n and n > 0 and n <= 200000, n
    step(f"{tag}: waterfall, contours, surface" + (", sticks" if hr else "; sticks disabled"), modes)
    def point():
        # the highest cell of the grid: project it, move there, read it
        c = pg.evaluate("""(()=>{const m=window.__m3d,V=m.V,g=V.g;let b=-1,bi=0;for(let i=0;i<V.raw.length;i++)if(V.raw[i]>b){b=V.raw[i];bi=i}
          const i=Math.floor(bi/g.nmz),j=bi%g.nmz,q=m.project(m.px(i),m.heightAt(i,j)*m.box().y,m.pz(j)),r=document.querySelector('.m3d-gl').getBoundingClientRect();
          return {x:r.left+q[0],y:r.top+q[1],i,j,v:b,rt:m.rtAt(i),mz:m.mzAt(j)}})()""")
        pg.mouse.move(c["x"], c["y"]); pg.wait_for_timeout(400)
        cur = pg.evaluate("window.__m3d.V.cur")
        assert abs(cur["i"] - c["i"]) <= 2 and abs(cur["j"] - c["j"]) <= 2, (cur, c)
        txt = pg.inner_text(".m3d-read"); assert "RT" in txt and "m/z" in txt, txt
        # coherent with the 2D map: the highest cell of the region has the same value in the 2D map at the same RT and m/z
        v2 = pg.evaluate(f"MAPPA.valueAt({M}, {c['rt']}, {c['mz']})")
        assert v2 is not None and abs(v2 - c["v"]) / c["v"] < 0.02 or v2 >= c["v"] * 0.98, (v2, c)
        assert pg.locator(".m3d-xic").count() == 1 and pg.locator(".m3d-spc").count() == 1
        pg.screenshot(path=SH + f"m3d_{tag}_point.png")
    step(f"{tag}: pointing reads RT, m/z and intensity coherent with the 2D map", point)
    def fix():
        n0 = pg.evaluate(f"MAPPA.pts({M}).length"); box = pg.locator(".m3d-gl").bounding_box()
        c = pg.evaluate("""(()=>{const m=window.__m3d,V=m.V,g=V.g;let b=-1,bi=0;for(let i=0;i<V.raw.length;i++)if(V.raw[i]>b){b=V.raw[i];bi=i}
          const i=Math.floor(bi/g.nmz),j=bi%g.nmz,q=m.project(m.px(i),m.heightAt(i,j)*m.box().y,m.pz(j)),r=document.querySelector('.m3d-gl').getBoundingClientRect();return [r.left+q[0],r.top+q[1]]})()""")
        pg.mouse.click(c[0], c[1]); attendi(pg, "window.__m3d.V.cur != null"); pg.evaluate("new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))"); assert pg.evaluate(f"MAPPA.pts({M}).length") == n0, "a click alone only fixes the point"
        pg.evaluate(f"MAPPA.st({M}).mark = true")                                                # «Mark» on: the click adds it
        pg.mouse.click(c[0], c[1]); pg.wait_for_function(f"MAPPA.pts({M}).length==={n0 + 1}", timeout=10000)
    step(f"{tag}: a click fixes a point (same list as the 2D map)", fix)
    def keys():
        i0 = pg.evaluate("window.__m3d.V.cur.i"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(150)
        assert pg.evaluate("window.__m3d.V.cur.i") == i0 + 1 and pg.evaluate("window.__m3d.V.plane") is True
        pg.fill("dialog.m3d [data-m=ti]", "5"); pg.dispatch_event("dialog.m3d [data-m=ti]", "input"); pg.wait_for_timeout(200)
        assert pg.evaluate("window.__m3d.V.cur.i") == 5
    step(f"{tag}: arrows and the time slider move the cursor and the plane", keys)
    def closes():
        pg.keyboard.press("Escape"); pg.wait_for_timeout(500)
        assert pg.evaluate("!document.querySelector('dialog.m3d')")
        assert pg.evaluate("document.activeElement && document.activeElement.dataset && document.activeElement.dataset.o==='view'"), "the focus goes back to the button"
        assert pg.locator(".pnl.map .mztab tbody tr").count() >= 1, "the point is in the table of the 2D map"
    step(f"{tag}: Esc closes, the focus goes back to the button, the point is in the 2D table", closes)


try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="it-IT", viewport={"width": 1440, "height": 950}).new_page()
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); pg.wait_for_timeout(1200)
        viewer(pg, "lr", False)
        def diff():
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.locator(BTN).click()
            pg.wait_for_function("window.__m3d && window.__m3d.V.g", timeout=60000)
            k = pg.evaluate("E.files.filter(f=>f.kind==='full')[1].k")
            pg.select_option("dialog.m3d [data-m=ref]", str(k)); pg.wait_for_function("window.__m3d.V.signed===true && window.__m3d.V.g.ref!==undefined", timeout=60000); pg.wait_for_timeout(800)
            assert pg.evaluate(INK) > 4
            pg.screenshot(path=SH + "m3d_lr_diff.png")
            pg.check("dialog.m3d [data-m=bg]"); pg.wait_for_function("window.__m3d.V.g.bg===true", timeout=60000)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            pg.evaluate("localStorage.removeItem('qqq.mappa')")
        step("lr: comparison A - B (signed heights) and constant background removed", diff)
        # old keys of the previous 3D view are ignored without errors
        def oldkeys():
            pg.evaluate("localStorage.setItem('qqq.mappa', JSON.stringify({t3pal:'cividis',t3sig:2,t3thr:.1}))"); pg.reload(); pg.wait_for_timeout(1500)
            assert not r.errs, r.errs
        step("old t3* keys of qqq.mappa are ignored", oldkeys)
        b.close()
    r.close(); r2 = Run(port=8839, wd="/tmp/wd_mappa3d_hr")
    with sync_playwright() as p:
        pg = r2.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(500)
        pg.evaluate("reveal(addPanel('map', { k: E.files.find(f => f.kind === 'full').k }))"); ready(pg); pg.wait_for_timeout(1500)
        viewer(pg, "hr", True)
    r2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for s in steps: print(" ", s)
r.report()
