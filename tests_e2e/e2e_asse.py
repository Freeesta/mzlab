# e2e_asse: axis lock of the map gestures (Shift held, Ctrl/Cmd + Shift for a pan, or the latch X / Y): pan, zoom box, marked point, arrows (asse.js)
import json, os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8871, wd="/tmp/wd_asse")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.click("text=Carica dati"); ready(pg)
        open_kind(pg, "full"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, "E.panels.some(p=>p.type==='map'&&p._a)")
        MP = "E.panels.find(p=>p.type==='map')"
        z = lambda: pg.evaluate(f"[{MP}.zoom,{MP}.zoomY]")
        def zoomed():
            pg.evaluate(f"(()=>{{const p={MP},a=p._a;p.zoom=[a.x0+(a.x1-a.x0)*.25,a.x1-(a.x1-a.x0)*.25];p.zoomY=[a.y0+(a.y1-a.y0)*.25,a.y1-(a.y1-a.y0)*.25];draw(p)}})()")
            attendi(pg, f"{MP}._a&&{MP}._a.x0>{MP}._a.full[0]")
        def box():                                       # canvas rectangle in page pixels
            pg.evaluate(f"{MP}.cv.scrollIntoView({{block:'center'}})"); attendi(pg, f"{MP}.cv.getBoundingClientRect().height>0")
            return pg.evaluate(f"(()=>{{const r={MP}.cv.getBoundingClientRect();return [r.left,r.top,r.width,r.height]}})()")
        def drag(mods, dx, dy, mid=None, late=()):
            b = box(); x0, y0 = b[0] + b[2] * .5, b[1] + b[3] * .5
            for m in mods: pg.keyboard.down(m)
            pg.mouse.move(x0, y0); pg.mouse.down()
            for m in late: pg.keyboard.down(m)                  # pressed after the mouse button: Shift before it starts a pan
            pg.mouse.move(x0 + dx / 2, y0 + dy / 2, steps=3); pg.mouse.move(x0 + dx, y0 + dy, steps=3)
            if mid: mid()
            pg.mouse.up()
            for m in (*mods, *late): pg.keyboard.up(m)
        def latch(k):
            pg.evaluate(MP + ".cv.focus()"); pg.keyboard.press(k)
        def held_pan():
            zoomed(); z0 = z(); drag(["Shift", "Control"], 70, 12, lambda: attendi(pg, f"{MP}.rd.textContent.includes('Solo RT')"))
            z1 = z(); assert z1[0] != z0[0] and z1[1] == z0[1], ("held, mostly horizontal: only RT moves", z0, z1)
            drag(["Shift", "Control"], 12, 70); z2 = z(); assert z2[1] != z1[1] and z2[0] == z1[0], ("held, mostly vertical: only m/z moves", z1, z2)
            drag(["Shift"], 40, 40); z3 = z(); assert z3[0] != z2[0] and z3[1] != z2[1], ("free: both move", z2, z3)
        step("pan with Ctrl+Shift held: the axis is the dominant direction", held_pan)
        def latched_pan():
            zoomed(); z0 = z(); latch("x"); assert pg.evaluate("ASSE.fix") == "x"; attendi(pg, "document.getElementById('sb-live').textContent.length>0")
            drag(["Shift"], 40, 40); z1 = z(); assert z1[0] != z0[0] and z1[1] == z0[1], ("latch X: only RT moves", z0, z1)
            latch("y"); assert pg.evaluate("ASSE.fix") == "y"; drag(["Shift"], 40, 40); z2 = z(); assert z2[1] != z1[1] and z2[0] == z1[0], ("latch Y: only m/z moves", z1, z2)
            latch("y"); assert pg.evaluate("ASSE.fix") is None, "Y again frees the axis"
            latch("x"); latch("Escape"); assert pg.evaluate("ASSE.fix") is None, "Esc frees the axis"
        step("pan with the latch X / Y", latched_pan)
        def boxz():
            pg.evaluate(f"MAPPA.whole({MP})"); attendi(pg, f"{MP}.zoom===null"); z0 = z()
            latch("x"); drag([], 70, 50); z1 = z(); assert z1[0] is not None and z1[1] == z0[1], ("latch X: the box zooms RT only", z0, z1)
            latch("x"); pg.evaluate(f"MAPPA.whole({MP})"); attendi(pg, f"{MP}.zoom===null")
            drag([], 10, 50, late=["Shift"]); z2 = z(); assert z2[1] is not None and z2[0] is None, ("held, vertical: the box zooms m/z only", z2)
        step("zoom box in one direction", boxz)
        def marked():
            pg.evaluate(f"MAPPA.whole({MP})"); attendi(pg, f"{MP}.zoom===null")
            pg.evaluate(f"(()=>{{const p={MP},a=p._a;MAPPA.pts(p).push({{rt:(a.x0+a.x1)/2,mz:(a.y0+a.y1)/2,note:''}});MAPPA.redraw(p)}})()"); attendi(pg, f"MAPPA.pts({MP}).length>0")
            q0 = pg.evaluate(f"({{...MAPPA.pts({MP})[0]}})"); b = box(); sx, sy = pg.evaluate(f"[{MP}._a.X({q0['rt']}),{MP}._a.Y({q0['mz']})]")
            pg.mouse.move(b[0] + sx, b[1] + sy); pg.mouse.down(); pg.keyboard.down("Shift"); pg.mouse.move(b[0] + sx + 40, b[1] + sy + 8, steps=4); pg.mouse.move(b[0] + sx + 80, b[1] + sy + 16, steps=4); pg.mouse.up(); pg.keyboard.up("Shift")
            attendi(pg, f"MAPPA.pts({MP})[0].rt!=={q0['rt']}"); q1 = pg.evaluate(f"({{...MAPPA.pts({MP})[0]}})")
            assert q1["mz"] == q0["mz"], ("held, horizontal: the marked point keeps its m/z", q0, q1)
        step("marked point dragged along one axis", marked)
        def arrows():
            pg.evaluate(f"MAPPA.whole({MP})"); pt = lambda: pg.evaluate(f"JSON.stringify(MAPPA.point({MP}))")
            latch("x"); p0 = pt(); pg.keyboard.press("ArrowUp"); pg.keyboard.press("ArrowDown"); assert pt() == p0, "latch X: up / down do nothing"
            latch("x"); latch("y"); pg.keyboard.press("ArrowRight"); assert pt() == p0, "latch Y: left / right do nothing"
            pg.keyboard.press("ArrowUp"); attendi(pg, f"JSON.stringify(MAPPA.point({MP}))!==" + json.dumps(p0))
            assert pt() != p0, "latch Y: up moves the sight"
            latch("y")
        step("arrows with the latch", arrows)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
