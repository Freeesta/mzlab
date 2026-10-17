"""Cursor of the chromatogram: the dot on top of the line (mouse, finger on an iPad viewport, arrow keys, tap elsewhere) and the snap to the nearest scan
of the kind shown (DDA with MS1/MS2 filters): the message «nessuno scan» never appears while the file has scans."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_cursore.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("cursore")
r = Run(port=8861, wd="/tmp/wd_cursore")
CH = "E.panels.find(p => p.type === 'chrom' && p.tab === 'full')"
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1024, "height": 1366}, has_touch=True, locale="it-IT")
        pg = ctx.new_page(); pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); ready(pg)
        cdp = ctx.new_cdp_session(pg)
        def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": i} for i, (x, y) in enumerate(pts)]})
        def X(rt): return pg.evaluate(f"(() => {{ const p = {CH}, q = p.cv.getBoundingClientRect(); return q.left + p._a.X({rt}); }})()")
        def Ymid(): return pg.evaluate(f"(() => {{ const q = {CH}.cv.getBoundingClientRect(); return q.top + q.height * 0.5; }})()")
        pg.evaluate(f"{CH}.el.scrollIntoView({{block:'start'}})"); pg.wait_for_timeout(400)
        rts = pg.evaluate(f"{CH}._a.sr[0].x")
        def dot_loc(): 
            b = pg.locator(".pnl.chrom .clh:visible").first.bounding_box(); return b["x"] + b["width"] / 2, b["y"] + b["height"] / 2, b
        def tap(rt):
            T("touchStart", [(X(rt), Ymid())]); T("touchEnd", []); pg.wait_for_timeout(700); ready(pg)
        def appear():
            assert pg.locator("#tbar").count() == 0
            tap(rts[len(rts) // 2])
            h = pg.locator(".pnl.chrom .clh:visible").first
            assert h.get_attribute("role") == "slider" and h.get_attribute("aria-label") and h.get_attribute("aria-valuenow")
            bx, by, bb = dot_loc(); assert bb["width"] >= 14 and abs(bx - X(pg.evaluate(f"{CH}.cur"))) < 2
        step("a touch shows the line with its dot (slider, aria-label)", appear)
        def mouse_no_dot():
            c0 = pg.evaluate(f"{CH}.cur"); pg.mouse.move(300, 300); pg.mouse.move(X(rts[len(rts) // 4]), Ymid()); pg.wait_for_timeout(200)
            assert pg.locator(".pnl.chrom .clh:visible").count() == 0, "with the mouse there is no dot"
            pg.mouse.click(X(rts[len(rts) // 4]), Ymid()); ready(pg)
            c1 = pg.evaluate(f"{CH}.cur"); assert abs(c1 - rts[len(rts) // 4]) < 0.5 and abs(c1 - c0) > 1, (c0, c1)
            x = X(c1); pg.mouse.move(x, Ymid()); pg.mouse.down()
            for k in range(1, 9): pg.mouse.move(x + (X(c1 + 3) - x) * k / 8, Ymid()); pg.wait_for_timeout(20)
            pg.mouse.up(); ready(pg)
            c2 = pg.evaluate(f"{CH}.cur"); assert abs(c2 - (c1 + 3)) < 0.4, (c1, c2)
        step("mouse: no dot; click and drag move the line itself", mouse_no_dot)
        def trackpad():
            pg.evaluate(f"zoomTo({CH}, 8, 16)"); ready(pg); pg.wait_for_timeout(400)
            x, y = X(12), Ymid(); pg.mouse.move(x, y)
            pg.mouse.wheel(120, 0); pg.wait_for_timeout(500)
            z = pg.evaluate(f"{CH}.zoom"); assert z[0] > 8.05 and abs((z[1] - z[0]) - 8) < 1e-6, z
            pg.keyboard.down("Control"); pg.mouse.wheel(0, -200); pg.keyboard.up("Control"); pg.wait_for_timeout(500)
            z2 = pg.evaluate(f"{CH}.zoom"); assert (z2[1] - z2[0]) < 8 - 0.1, z2
            pg.evaluate(f"{CH}.zoom = null; draw({CH})"); ready(pg)
        step("trackpad: two fingers sideways pan, pinch / Ctrl+wheel zoom", trackpad)
        def touch_drag():
            tap(rts[len(rts) // 2])
            c0 = pg.evaluate(f"{CH}.cur"); bx, by, _ = dot_loc(); tx = X(c0 - 2)
            T("touchStart", [(bx, by)]); pg.wait_for_timeout(60)
            for k in range(1, 9): T("touchMove", [(bx + (tx - bx) * k / 8, by)]); pg.wait_for_timeout(30)
            T("touchEnd", []); ready(pg)
            c1 = pg.evaluate(f"{CH}.cur"); assert abs(c1 - (c0 - 2)) < 0.4, (c0, c1)
        step("a finger drags the dot (iPad viewport)", touch_drag)
        def tap_else():
            c0 = pg.evaluate(f"{CH}.cur"); tgt = rts[len(rts) // 5]; tap(tgt); pg.wait_for_timeout(200)
            c1 = pg.evaluate(f"{CH}.cur"); assert abs(c1 - tgt) < 0.5 and abs(c1 - c0) > 1, (c0, c1, tgt)
        step("a tap elsewhere places the cursor as always", tap_else)
        def keys():
            pg.locator(".pnl.chrom .clh:visible").first.focus(); c0 = pg.evaluate(f"{CH}.cur")
            pg.keyboard.press("ArrowRight"); ready(pg); c1 = pg.evaluate(f"{CH}.cur"); assert c1 > c0, (c0, c1)
            pg.keyboard.press("Shift+ArrowLeft"); ready(pg); c2 = pg.evaluate(f"{CH}.cur"); assert c2 < c1 - (c1 - c0) * 2, (c0, c1, c2)
        step("arrow keys on the dot move one scan (Shift: five)", keys)
        def snap():
            s1 = "E.panels.find(p => p.type === 'spec' && p.link === " + CH + ".id)"
            i = len(rts) // 3; mid = (rts[i] + rts[i + 1]) / 2
            pg.evaluate(f"(() => {{ const p = {CH}, s = {s1}; p.cur = {mid}; s.r0 = {mid} - 1e-6; s.r1 = {mid} + 1e-6; drawSpec(s); }})()"); ready(pg)
            cur = pg.evaluate(f"{CH}.cur"); a = pg.evaluate(f"!!({s1}._a)")
            assert a, "the spectrum is drawn (no «nessuno scan»)"
            ms1 = pg.evaluate("(async () => (await (await fetch('api/nearest_scan?k=0&rt=%s&level=1')).json()).rt)()" % mid)
            assert abs(cur - ms1) < 1e-6 and cur != mid, (cur, ms1, mid)
        step("a cursor between two scans snaps to the nearest scan of the shown kind", snap)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
