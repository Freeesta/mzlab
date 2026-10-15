"""Block A (charts): fixed m/z axis of the spectrum, lock = y only, zoom box / lines on the axes, no 'y x10', Excel on the TIC, XIC under its spectrum."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_assi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.link!=null)"
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
r = Run(port=8872, wd="/tmp/wd72")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        def rect(expr):   # canvas rectangle + margins of a panel
            return pg.evaluate(f"(()=>{{const p={expr},r=p.cv.getBoundingClientRect();p.el.scrollIntoView({{block:'center'}});const q=p.cv.getBoundingClientRect();return {{l:q.left,t:q.top,w:q.width,h:q.height,M:M}}}})()")
        def pt(expr, xs, ys):  # pixel of axis values
            return pg.evaluate(f"(()=>{{const p={expr},q=p.cv.getBoundingClientRect();return {{x:q.left+p._a.X({xs}),y:q.top+p._a.Y({ys})}}}})()")
        pg.evaluate(f"(()=>{{const p={CH};p.el.scrollIntoView({{block:'center'}})}})()"); pg.wait_for_timeout(400)
        c = pt(CH, "14.3", "0"); pg.mouse.click(c["x"], c["y"] - 40); pg.wait_for_timeout(1200)
        def x_fixed():
            a = pg.evaluate(f"(()=>{{const s={SPEC};return {{x0:s._a.x0,x1:s._a.x1,f:s._a.full}}}})()")
            assert a["x0"] == a["f"][0] and a["x1"] == a["f"][1], a
            for rt in (13.0, 15.5, 18.0):
                c = pt(CH, str(rt), "0"); pg.mouse.click(c["x"], c["y"] - 40); pg.wait_for_timeout(700)
                b = pg.evaluate(f"(()=>{{const s={SPEC};return {{x0:s._a.x0,x1:s._a.x1}}}})()")
                assert abs(b["x0"] - a["x0"]) < 1 and abs(b["x1"] - a["x1"]) < 1, (a, b)      # the axis is the range of the FILE: another file of the series (synthetic ones differ by a few mDa) may differ slightly
        step("spectrum: x axis is the same for every scan (whole range of the file)", x_fixed)
        def lock_y_only():
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(500)
            assert pg.evaluate(f"!{SPEC}.lock"), "open by default, the arrows do not close it"
            pg.click(".pnl.spec [data-a=lock]"); pg.wait_for_timeout(400)
            assert pg.evaluate(f"!!{SPEC}.lock")
            lk = pg.evaluate(f"(()=>{{const s={SPEC};return Object.keys(s.lock)}})()"); assert "x0" not in lk and "ymax" in lk, lk
            b = pg.evaluate(f"(()=>{{const s={SPEC},e=s.el.querySelector('[data-a=lock]'),c=s.cv.getBoundingClientRect(),r=e.getBoundingClientRect();return {{bar:!!e.closest('.tbs,.hmore'),pressed:e.getAttribute('aria-pressed'),on:e.classList.contains('on'),label:!!e.getAttribute('aria-label'),above:r.bottom<=c.top+1}}}})()")
            assert b["bar"] and b["pressed"] == "true" and b["on"] and b["label"] and b["above"], b          # in the button bar, never over the plot or the y axis numbers
        step("lock: y only, button in the panel bar", lock_y_only)
        def no_yz():
            assert pg.evaluate("document.querySelectorAll('[data-a=yz]').length") == 0
        step("no 'y x10' button", no_yz)
        def box_xy():
            a0 = pg.evaluate(f"(()=>{{const p={CH};return {{x0:p._a.x0,x1:p._a.x1,ym:p._a.ymax}}}})()")
            pg.click(".pnl.chrom [data-a=izoom] >> nth=0"); pg.wait_for_timeout(200)
            u = pt(CH, "14.0", "p._a.ymax*0.9"); v = pt(CH, "14.6", "p._a.ymax*0.2")
            pg.mouse.move(u["x"], u["y"]); pg.mouse.down(); pg.mouse.move(v["x"], v["y"], steps=5); pg.mouse.up(); pg.wait_for_timeout(600)
            a = pg.evaluate(f"(()=>{{const p={CH};return {{z:p.zoom,zy:p.zoomY,ym:p._a.ymax}}}})()")
            assert a["z"] and abs(a["z"][0] - 14.0) < 0.1 and abs(a["z"][1] - 14.6) < 0.1, a
            assert a["zy"] and abs(a["ym"] - 0.9 * a0["ym"]) / a0["ym"] < 0.05 and a["zy"][0] > 0.1 * a0["ym"], (a, a0)
            pg.click(".pnl.chrom [data-a=izoom] >> nth=0")                                    # lens off
        step("lens + box zooms x and y together", box_xy)
        def line_y():
            pg.click(".pnl.chrom [data-a=fit] >> nth=0"); pg.wait_for_timeout(300)
            x0 = pg.evaluate(f"{CH}._a.x0"); ym = pg.evaluate(f"{CH}._a.ymax")
            u = pt(CH, "p._a.x0", "p._a.ymax*0.8"); v = pt(CH, "p._a.x0", "p._a.ymax*0.1")
            pg.mouse.move(u["x"] - 30, u["y"]); pg.mouse.down(); pg.mouse.move(v["x"] - 30, v["y"], steps=5)
            assert pg.evaluate(f"(()=>{{const z={CH}.el.querySelector('.zl');return !z.hidden&&z.classList.contains('v')}})()"), "a vertical line while dragging"
            pg.mouse.up(); pg.wait_for_timeout(500)
            a = pg.evaluate(f"(()=>{{const p={CH};return {{z:p.zoom,zy:p.zoomY,x0:p._a.x0}}}})()")
            assert a["z"] is None and a["zy"] and abs(a["zy"][1] - 0.8 * ym) / ym < 0.06, a
        step("drag on the numbers left of the y axis: line, only y", line_y)
        def line_x():
            pg.click(".pnl.chrom [data-a=fit] >> nth=0"); pg.wait_for_timeout(300)
            b = rect(CH); u = pt(CH, "14.0", "0"); v = pt(CH, "14.8", "0"); yy = b["t"] + b["h"] - 14
            pg.mouse.move(u["x"], yy); pg.mouse.down(); pg.mouse.move(v["x"], yy, steps=5)
            assert pg.evaluate(f"(()=>{{const z={CH}.el.querySelector('.zl');return !z.hidden&&z.classList.contains('h')}})()"), "a horizontal line while dragging"
            pg.mouse.up(); pg.wait_for_timeout(500)
            a = pg.evaluate(f"(()=>{{const p={CH};return {{z:p.zoom,zy:p.zoomY}}}})()")
            assert a["z"] and abs(a["z"][0] - 14.0) < 0.1 and abs(a["z"][1] - 14.8) < 0.1 and a["zy"] is None, a
            pg.click(".pnl.chrom [data-a=fit] >> nth=0")
        step("drag on the numbers under the x axis: line, only x", line_x)
        def spec_zoom():
            s0 = pg.evaluate(f"(()=>{{const s={SPEC};return {{ym:s._a.ymax,x0:s._a.x0,x1:s._a.x1}}}})()")
            YM = "Math.min(p._a.ymax,p._a.yfull[1])"      # the intensity lock can keep a top higher than the automatic one: the box must be inside the automatic range to zoom y
            u = pt(SPEC, "p._a.x0+(p._a.x1-p._a.x0)*0.3", YM + "*0.6"); v = pt(SPEC, "p._a.x0+(p._a.x1-p._a.x0)*0.5", YM + "*0.05")
            pg.mouse.move(u["x"], u["y"]); pg.mouse.down(); pg.mouse.move(v["x"], v["y"], steps=5); pg.mouse.up(); pg.wait_for_timeout(700)
            a = pg.evaluate(f"(()=>{{const s={SPEC};return {{z:s.zoom,zy:s.zoomY,x0:s._a.x0,x1:s._a.x1}}}})()")
            assert a["z"] and a["zy"] and a["x0"] > s0["x0"] and a["x1"] < s0["x1"], a
            pg.click(".pnl.spec [data-a=fit]"); pg.wait_for_timeout(400)
            assert pg.evaluate(f"{SPEC}.zoomY") is None and pg.evaluate(f"{SPEC}.zoom") is None
        step("spectrum: box zoom x+y, reset", spec_zoom)
        def old_notebook():
            r0 = pg.evaluate("(()=>{const o={yz:10,zoomY:null};return o.yz})()")
            n = pg.evaluate(f"(()=>{{const p=addPanel('xic',{{yz:10,traces:[]}});return p.zoomY}})()"); assert n is None, n
        step("an old 'yz' in a notebook is ignored", old_notebook)
        def excel_tic():
            pg.evaluate("E.files.forEach(f=>f.vis=true); redrawAll()")
            with pg.expect_download() as d: dlmenu(pg, ".pnl.chrom", "xlsx")
            f = d.value; assert f.suggested_filename.endswith(".xlsx"), f.suggested_filename
            rows = xlsx_rows(f.path()); head = [c[1] for c in rows[0] if c]
            assert all(h.startswith("RT (min)") or h.startswith("Intensità (cps)") for h in head) and sum(h.startswith("Intensità") for h in head) == 2, head   # one intensity column per file
            assert all(c[0] == "n" for c in rows[1][:3] if c)
        step("Excel on the total chromatogram (RT + one column per file)", excel_tic)
        def xic_below():
            sp = pg.evaluate(f"(()=>{{const s={SPEC};return {{y:s.y,h:s.h,id:s.id}}}})()")
            n0 = pg.evaluate("E.panels.length"); pg.evaluate(f"openXic(null, {{mz:364.4, obs:true, after:{SPEC}}})"); pg.wait_for_timeout(300)
            pg.click("#xic-go"); ready(pg)
            xp = pg.evaluate("(()=>{const x=E.panels.find(p=>p.type==='xic'&&p.traces.length===1);return {y:x.y,full:x.full}})()")
            assert abs(xp["y"] - (sp["y"] + sp["h"] + pg.evaluate("pnlGap()"))) < 3, (xp, sp)
        step("XIC from the spectrum menu sits right under that spectrum", xic_below)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
