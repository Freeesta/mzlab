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
        def small_peak():                                                      # at RT 14.3 the 386 peak is 3 % of the parent (364): under the old 5 % rule it had no label
            pos = pg.evaluate(f"(()=>{{const a={M}._a,r={M}.cv.getBoundingClientRect();return [r.left+a.X(14.3),r.top+a.Y(364.3)]}})()")
            pg.mouse.move(pos[0], pos[1]); attendi(pg, f"({M}._mz.lbls||[]).some(l=>+l.t>380&&+l.t<392)")
            L = pg.evaluate(f"({M}._mz.lbls||[]).map(l=>({{...l}}))"); hi = [l for l in L if 380 < float(l["t"]) < 392]
            assert hi, ("no label for the small peak near 386", [l["t"] for l in L])
            r = pg.evaluate(f"{M}._mz.sp.getBoundingClientRect().toJSON()"); l = hi[0]
            pg.mouse.move(r["x"] + l["x"] + l["w"] / 2, r["y"] + l["y"] + l["h"] / 2); attendi(pg, f"{M}._mz.sp.title.includes('S/N')")
            tip = pg.evaluate(f"{M}._mz.sp.title"); assert "S/N" in tip and l["t"] in tip, tip
        step("a small peak at higher m/z has its label, with its S/N in the tooltip", small_peak)
        def density():
            n = {}
            for d in ("few", "mid", "many"):
                pg.evaluate(f"(() => {{ const s = {M}.el.querySelector('[data-mz=dens]'); s.value = '{d}'; s.dispatchEvent(new Event('change')); }})()")
                n[d] = len(pg.evaluate(f"({M}._mz.lbls||[])"))
            assert n["few"] <= 4 and n["few"] <= n["mid"] <= 8 and n["mid"] <= n["many"] <= 14, n
        step("the density option changes the number of labels", density)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
