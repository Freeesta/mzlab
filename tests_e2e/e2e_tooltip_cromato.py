"""Hover box of a chromatogram (mouse and finger on an iPad viewport): the retention time it prints is the one under the cursor, nothing (marker, bar handle,
cursor dot) covers its first digit, and the box stays off the point, off the x axis with its numbers and title, and inside the panel, also with an integration window."""
# covers: mzlab/web/grafici.js, mzlab/web/app.css, mzlab/web/barre-maniglie.js
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds; import numpy as np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tooltip_cromato.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "tooltip_cromato_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.full_scan(D / "t5.mzML", 5, np.random.default_rng(1)); ds.full_scan(D / "t15.mzML", 15, np.random.default_rng(2))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8987, wd="/tmp/wd_tooltip_cromato")
# geometry of the panel, the hover box and everything that must stay clear of it (all in page pixels)
GEO = f"""(() => {{ const p = {XP}, a = p._a, c = p.cv.getBoundingClientRect(), t = p.tip.getBoundingClientRect(), box = e => {{ const q = e.getBoundingClientRect(); return [q.left, q.top, q.right, q.bottom]; }};
  return {{ hidden: p.tip.hidden, text: p.tip.innerText, tip: [t.left, t.top, t.right, t.bottom], panel: box(p.el), axis: c.top + a.H - M.b, first: p.tip.querySelector('b').innerText,
           x0: a.x0, x1: a.x1, W: a.W, ml: M.l, mr: M.r, left: c.left, xs: a.sr[0].x }}; }})()"""
def overlap(u, v): return u[0] < v[2] and v[0] < u[2] and u[1] < v[3] and v[1] < u[3]
def check(pg, px, py, touch, what):
    g = pg.evaluate(GEO); assert not g["hidden"], (what, "no hover box")
    rt = g["x0"] + (px - g["left"] - g["ml"]) / (g["W"] - g["ml"] - g["mr"]) * (g["x1"] - g["x0"])          # the RT under the cursor, from the pixel and the axis
    assert g["first"] == f"{rt:.2f} min", (what, g["first"], f"{rt:.2f} min")
    near = min(g["xs"], key=lambda v: abs(v - rt)); assert abs(near - rt) <= (g["xs"][1] - g["xs"][0]) / 2 + 1e-9, (what, near, rt)    # and it is a point of the data, to within half a sample
    t = g["tip"]; assert t[0] >= g["panel"][0] and t[2] <= g["panel"][2] and t[1] >= g["panel"][1], (what, "outside the panel", t, g["panel"])
    assert t[3] <= g["axis"], (what, "covers the x axis, its numbers or its title", t, g["axis"])
    assert not overlap(t, [px - 8, py - 8, px + 8, py + 8]), (what, "covers the point", t, px, py)
    if touch: assert not overlap(t, [px - 22, py - 22, px + 22, py + 22]), (what, "under the finger", t, px, py)
    return g
try:
    with sync_playwright() as p:
        for touch in (False, True):
            pg = r.page(p, touch=touch, viewport={"width": 1024, "height": 1366}); ctx = pg.context; tag = "touch" if touch else "mouse"
            pg.set_input_files("#pick", [str(D / "t5.mzML"), str(D / "t15.mzML")]); pg.wait_for_selector("text=Load data"); pg.click("text=Load data"); ready(pg)
            pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); wait_for_js(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
            pg.evaluate(f"{XP}.el.scrollIntoView({{block:'center'}})"); frames(pg, 3)
            if touch: pg.evaluate("document.documentElement.classList.add('ptr-touch');document.documentElement.classList.remove('ptr-mouse')")
            cdp = ctx.new_cdp_session(pg) if touch else None
            def at(rt, fy):        # page pixel of an RT and a fraction of the plot height (1 = on the baseline)
                return pg.evaluate(f"(() => {{ const p = {XP}, a = p._a, q = p.cv.getBoundingClientRect(); return [q.left + a.X({rt}), q.top + M.t + (a.H - M.t - M.b) * {fy} - (({fy}) >= 1 ? 3 : 0)]; }})()")
            def point(rt, fy, what):
                x, y = at(rt, fy)
                if touch:
                    cdp.send("Input.dispatchTouchEvent", {"type": "touchStart", "touchPoints": [{"x": x, "y": y, "id": 0}]}); frames(pg, 3)
                    pg.mouse.move(x, y)          # the compatibility mouse event a finger sends; the box must be up for it
                else: pg.mouse.move(x - 20, y); pg.mouse.move(x, y)
                frames(pg, 3); g = check(pg, x, y, touch, f"{tag} {what}")
                if touch: cdp.send("Input.dispatchTouchEvent", {"type": "touchEnd", "touchPoints": []})
                return g
            def inside(): [point(rt, fy, f"rt {rt} fy {fy}") for rt in (1.3, 8.3, 14.3) for fy in (0.1, 0.5, 1)]
            step(f"{tag}: the RT is the one under the cursor, the box is off the point and above the axis", inside)
            def edges(): [point(rt, fy, f"edge rt {rt} fy {fy}") for rt in (0.2, 15.8) for fy in (0.05, 1)]
            step(f"{tag}: near the left and right edges and the top the box stays in the panel", edges)
            def window():
                pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode='man';p.intf=p._a.sr[0].key;cursorLine(p)}})()")
                c = pg.evaluate(f"(()=>{{const p={XP},a=p._a,q=p.cv.getBoundingClientRect();return {{l:q.left,t:q.top,a:a.X(13.9),b:a.X(14.7),y:a.H-M.b-6}}}})()")
                pg.mouse.move(c["l"] + c["a"], c["t"] + c["y"]); pg.mouse.down(); pg.mouse.move(c["l"] + c["b"], c["t"] + c["y"], steps=8); pg.mouse.up(); frames(pg, 3)
                assert pg.evaluate(f"{XP}.ints.length") == 1 and pg.evaluate(f"{XP}.el.querySelectorAll('.ibh').length") == 2
                x0 = pg.evaluate(f"{XP}.ints[0].a")
                for rt in (x0 + 0.05, x0 + 0.1):     # next to the left bar, where its 44 px handle (a transparent circle) lies under the box
                    point(rt, 1, f"next to the bar {rt:.2f}")
                    z = pg.evaluate(f"(() => {{ const p = {XP}, zi = e => +getComputedStyle(e).zIndex; return [...p.el.querySelectorAll('.ibh, .clh')].every(h => zi(p.tip) > zi(h)); }})()")      # same stacking context, so a higher z-index draws over
                    assert z, "a bar handle or the cursor dot is drawn over the hover box"
            step(f"{tag}: with an integration window the bar handles never draw over the box", window)
            r.b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
