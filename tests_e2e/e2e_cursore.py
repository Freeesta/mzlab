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
        pg = ctx.new_page(); pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
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
        def appear():
            assert pg.locator("#tbar").count() == 0
            x, y = X(rts[len(rts) // 2]), Ymid(); pg.mouse.click(x, y); ready(pg)
            h = pg.locator(".pnl.chrom .clh:visible").first
            assert h.get_attribute("role") == "slider" and h.get_attribute("aria-label") and h.get_attribute("aria-valuenow")
            bx, by, bb = dot_loc(); assert bb["width"] >= 14 and abs(bx - X(pg.evaluate(f"{CH}.cur"))) < 2
        step("a click shows the line with its dot (slider, aria-label)", appear)
        def mouse_drag():
            c0 = pg.evaluate(f"{CH}.cur"); bx, by, _ = dot_loc(); tx = X(c0 + 3)
            pg.mouse.move(bx, by); pg.mouse.down()
            for k in range(1, 9): pg.mouse.move(bx + (tx - bx) * k / 8, by); pg.wait_for_timeout(20)
            pg.mouse.up(); ready(pg)
            c1 = pg.evaluate(f"{CH}.cur"); assert abs(c1 - (c0 + 3)) < 0.4, (c0, c1)
        step("the mouse drags the dot", mouse_drag)
        def touch_drag():
            c0 = pg.evaluate(f"{CH}.cur"); bx, by, _ = dot_loc(); tx = X(c0 - 2)
            T("touchStart", [(bx, by)]); pg.wait_for_timeout(60)
            for k in range(1, 9): T("touchMove", [(bx + (tx - bx) * k / 8, by)]); pg.wait_for_timeout(30)
            T("touchEnd", []); ready(pg)
            c1 = pg.evaluate(f"{CH}.cur"); assert abs(c1 - (c0 - 2)) < 0.4, (c0, c1)
        step("a finger drags the dot (iPad viewport)", touch_drag)
        def tap_else():
            c0 = pg.evaluate(f"{CH}.cur"); tgt = rts[len(rts) // 5]; T("touchStart", [(X(tgt), Ymid())]); T("touchEnd", []); pg.wait_for_timeout(900); ready(pg)
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
