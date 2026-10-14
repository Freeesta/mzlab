"""Touch-ups of the 2D RT x m/z map: one label per group of peaks in the side spectrum, intensity axis of the XIC under the map, short legend of the difference,
sample selector (also when the map follows the bar), no padlock button, the map keeps its size when points are marked, no «Data» buttons on the margins."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8902, wd="/tmp/wd_mapritocchi_e2e")
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
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); pg.wait_for_timeout(600)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); pg.wait_for_timeout(900); ready(pg)
        c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        def hover(fx, fy):
            pg.mouse.move(c[0] + c[2] * fx, c[1] + c[3] * fy); pg.wait_for_timeout(250); ready(pg); pg.wait_for_timeout(250)
        def one_per_group():
            seen = 0
            for fx in (.2, .4, .6, .8):
                for fy in (.3, .5, .7):
                    hover(fx, fy)
                    mz = sorted(float(l["t"]) for l in pg.evaluate(f"({M}._mz.lbls||[]).map(l=>({{t:l.t}}))"))
                    seen += len(mz)
                    for a, q in zip(mz, mz[1:]): assert q - a > 2.5, (a, q, mz)             # no isotope or lower neighbour of a labelled peak
            assert seen, "no label drawn"
        step("side spectrum: labels only on the highest peak of each group", one_per_group)
        def xic_axis():
            hover(.5, .5)
            n = pg.evaluate(f"(()=>{{const s={M}._mz.xc,g=s.getContext('2d'),d=s.width/s.getBoundingClientRect().width,w=Math.round(M.l*d)-2,h=s.height,px=g.getImageData(0,0,w,h).data;let k=0;for(let i=3;i<px.length;i+=4)if(px[i]>0)k++;return k}})()")
            assert n > 30, n                                                                  # tick labels in the left margin
        step("XIC under the map: intensity axis at the left", xic_axis)
        def no_data_buttons():
            assert pg.locator(".pnl.map .tdbtn").count() == 0
            assert pg.evaluate(f"{M}._mz.sp.tabIndex") == 0 and pg.evaluate(f"{M}._mz.xc.tabIndex") == 0       # the table stays on T of the focused chart
        step("no «Data» buttons on the axes", no_data_buttons)
        def no_lock_btn():
            assert pg.locator(".pnl.map [data-nav=maplock]").count() == 0
        step("no «Lock the sight» button on the 2D map", no_lock_btn)
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); pg.wait_for_timeout(900); ready(pg)          # back to the page: the panel grows only outside full screen
        def selector():
            def sel(): return pg.locator(".pnl.map select[data-o=k]")
            assert sel().count() == 1 and sel().is_enabled()
            assert [o for o in sel().locator("option").all_inner_texts()] == ["FullScan_t0", "FullScan_t30"], sel().locator("option").all_inner_texts()
            sel().select_option(index=1); pg.wait_for_timeout(600); ready(pg)
            assert pg.evaluate(f"E.files[{M}.k].label") == "FullScan_t30"
            pg.evaluate("soloFile(E.files[E.cur])"); attendi(pg, 'E.browse'); ready(pg)           # one file shown: the map follows the bar, the selector still works
            assert sel().is_enabled()
            sel().select_option(index=0); pg.wait_for_timeout(700); ready(pg)
            assert pg.evaluate("E.cur") == pg.evaluate(f"{M}.k") or pg.evaluate("E.files[E.cur].label") == "FullScan_t0", pg.evaluate("E.cur")
            assert pg.evaluate(f"E.files[E.cur].label") == "FullScan_t0"
            pg.evaluate("tabFiles().forEach(f=>{f.vis=true});renderFileList();renderNav();redrawAll()"); ready(pg)
        step("sample selector in the map panel (also when the map follows the bar)", selector)
        def mark_keeps_size():
            h = lambda: pg.evaluate(f"{M}.cv.getBoundingClientRect().height")
            pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
            a = pg.evaluate(f"({M}._a.x0+{M}._a.x1)/2"); y0, y1 = pg.evaluate(f"[{M}._a.y0,{M}._a.y1]")
            def add(n): pg.evaluate(f"(async()=>{{const p={M};for(let i=0;i<{n};i++)await MAPPA.addPointAt(p,p._a.x0+(p._a.x1-p._a.x0)*(0.1+0.8*Math.random()),p._a.y0+(p._a.y1-p._a.y0)*(0.1+0.8*Math.random()),1)}})()"); pg.wait_for_timeout(1500); ready(pg)
            add(1); pg.wait_for_selector(".pnl.map .mztab tbody tr"); pg.wait_for_timeout(500); h1 = h()
            add(14); pg.wait_for_timeout(1500); h2 = h()
            assert pg.locator(".pnl.map .mztab tbody tr").count() >= 10
            assert abs(h1 - h2) <= 2, (h1, h2)                                                  # same map size with 1 or 15 points
            sc = pg.evaluate("(()=>{const e=document.querySelector('.pnl.map .mzt-sc');return [e.scrollHeight,e.clientHeight,getComputedStyle(e).overflowY]})()")
            assert sc[0] > sc[1] and sc[2] in ("auto", "scroll"), sc                             # the table scrolls
            pg.click(".pnl.map [data-t=clear]"); pg.wait_for_timeout(300)
            if pg.locator("dialog[open] button").count(): pg.keyboard.press("Enter")
            pg.wait_for_timeout(800)
        step("marking many points: the map keeps its size, the table below scrolls", mark_keeps_size)
        def legend():
            pg.evaluate(f"(()=>{{const p={M};p.corr='f'+E.files[0].k;p.ref=E.files[0].k;p.k=E.files[1].k;draw(p)}})()"); pg.wait_for_timeout(1500); ready(pg)
            t = pg.evaluate(f"{M}.leg.innerText"); assert "▲ more in the sample" in t and "▼ more in the reference" in t and "e+" not in t and "orange" not in t, t
            assert len(t) < 140, t
        step("difference map: short legend without a numeric range", legend)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
