"""Stacked chromatograms of a view (TIC + XIC): same time range, same left and right edge of the plot (± 1 px) at three window widths and after a zoom;
zoom and cursor linked by default, one panel can be unlinked; the y axes stay independent."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_asse_x.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8864, wd="/tmp/wd_assex")
LINES = "E.panels.filter(p=>['chrom','xic'].includes(p.type)&&p.tab==='full')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("openXic(null,{mz:364.37,obs:true})"); pg.click("#xic-go"); ready(pg); pg.wait_for_timeout(1200)
        edges = lambda: pg.evaluate(f"{LINES}.map(p=>{{const r=p.cv.getBoundingClientRect(),a=p._a;return [r.left+a.X(a.x0),r.left+a.X(a.x1),a.x0,a.x1,p.type]}})")
        def same(tag):
            e = edges(); assert len(e) >= 2, e
            for q in e[1:]:
                assert abs(q[0] - e[0][0]) <= 1 and abs(q[1] - e[0][1]) <= 1, (tag, e)
                assert abs(q[2] - e[0][2]) < 1e-9 and abs(q[3] - e[0][3]) < 1e-9, (tag, e)
        def widths():
            for w in (1280, 1440, 1680):
                pg.set_viewport_size({"width": w, "height": 2200}); pg.wait_for_timeout(1200); ready(pg); same(w)
        step("left and right edges and time range coincide at three widths", widths)
        def zoom():
            pg.evaluate(f"zoomTo({LINES}[0], 12, 16)"); ready(pg); pg.wait_for_timeout(600)
            e = edges(); same("zoom"); assert abs(e[0][2] - 12) < 1e-6 and abs(e[1][2] - 12) < 1e-6 and abs(e[1][3] - 16) < 1e-6, e
        step("a zoom in one panel is the zoom of the other", zoom)
        def cursor():
            pg.evaluate(f"(()=>{{const p={LINES}[0];p.cur=14.2;draw(p)}})()"); ready(pg); pg.wait_for_timeout(500)
            c = pg.evaluate(f"{LINES}.map(p=>p.cur)"); assert all(abs(x - 14.2) < 1e-6 for x in c), c
            xs = pg.evaluate(f"{LINES}.map(p=>{{const r=p.cl.getBoundingClientRect();return r.left}})"); assert abs(xs[0] - xs[1]) <= 1, xs
        step("one cursor at the same x in both panels", cursor)
        def unlink():
            pg.evaluate(f"toggleTl({LINES}[1])"); pg.evaluate(f"zoomTo({LINES}[0], 13, 15)"); ready(pg); pg.wait_for_timeout(500)
            e = edges(); assert abs(e[1][2] - 12) < 1e-6, e
            assert abs(e[1][0] - e[0][0]) <= 1, "the plot area stays aligned even if unlinked"
        step("a panel can be unlinked (its zoom stays; the edges stay aligned)", unlink)
        def ydep():
            ym = pg.evaluate(f"{LINES}.map(p=>p._a.ymax)"); assert ym[0] != ym[1], ym
        step("the y axes are independent", ydep)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
