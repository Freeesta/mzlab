"""Labels of the spectrum drawn at the side of the RT x m/z map (mappa.js, drawSide): no label touches the bar of its own peak, none sits on another,
none leaves the panel; the labels that do not fit beyond the tip go to the left of it (above the bar)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8901, wd="/tmp/wd_mapetich_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:900]))
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); pg.wait_for_timeout(600)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); pg.wait_for_timeout(900); ready(pg)
        c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        def labels():
            seen = []
            for fx in (.2, .35, .5, .65, .8):
                for fy in (.3, .5, .7):
                    pg.mouse.move(c[0] + c[2] * fx, c[1] + c[3] * fy); pg.wait_for_timeout(250); ready(pg); pg.wait_for_timeout(250)
                    L = pg.evaluate(f"({M}._mz.lbls||[]).map(l=>({{...l}}))"); W = pg.evaluate(f"{M}._mz.sp.getBoundingClientRect().width")
                    for l in L:
                        if l["right"]: assert l["x"] >= l["tip"] + 3, l                       # beyond the tip, never on the bar
                        else: assert l["y"] + l["h"] <= l["ly"] - 1, l                        # above the bar
                        assert l["x"] >= 0 and l["x"] + l["w"] <= W, (l, W)
                    for i, a in enumerate(L):
                        for q in L[i + 1:]:
                            assert a["y"] + a["h"] <= q["y"] or q["y"] + q["h"] <= a["y"] or a["x"] + a["w"] <= q["x"] or q["x"] + q["w"] <= a["x"], (a, q)
                    seen += L
            assert seen, "no label drawn"
            return seen
        step("side spectrum labels do not touch their bar, each other or the edge", labels)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
