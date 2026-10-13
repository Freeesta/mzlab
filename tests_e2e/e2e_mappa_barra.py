"""Controls of the RT x m/z map: they sit in the toolbar of the panel head (same .bt size as the others), nothing floats over the plot, there is no
«How it is extracted» button, and zoom, reset, lock (L) and full screen still work."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8903, wd="/tmp/wd_mapbarra_e2e")
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + repr(e)[:600] + " | menu=" + str(pg.evaluate("document.querySelector('#ctx').innerText")[:300])))
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
        settle = lambda: (ready(pg), pg.wait_for_timeout(500))
        shown = lambda: pg.evaluate(f"(()=>{{const s={M}._mz.shown;return s&&s.sp?{{rt:s.pt.rt,mz:s.pt.mz,sid:s.sp.sid}}:null}})()")
        lock = lambda: pg.evaluate(f"!!{M}._mz.lock")
        menu_open = lambda: pg.evaluate("!document.querySelector('#ctx').hidden")
        settle = lambda: (ready(pg), pg.wait_for_timeout(400))
        def place():
            r = pg.evaluate("""(()=>{const el=document.querySelector('.pnl.map'),hd=el.querySelector('.hd').getBoundingClientRect(),cv=el.querySelector('canvas').getBoundingClientRect(),
              bs=[...el.querySelectorAll('.mznav button')].filter(b=>!b.hidden),o=[...el.querySelectorAll('.hd .tbg:not(.mznav) .bt')].filter(b=>!b.hidden);
              const ov=bs.map(b=>b.getBoundingClientRect()).some(r=>r.left<cv.right&&r.right>cv.left&&r.top<cv.bottom&&r.bottom>cv.top);
              return {n:bs.length,inHead:bs.every(b=>b.closest('.hd')),overlap:ov,floating:bs.some(b=>b.closest('.mzgrid')),
                h:bs.map(b=>b.getBoundingClientRect().height),ho:o.map(b=>b.getBoundingClientRect().height),names:bs.map(b=>b.dataset.nav),explain:!!el.querySelector('[data-nav=mapexplain]'),
                row:bs.every(b=>{const r=b.getBoundingClientRect();return r.top>=hd.top-1&&r.bottom<=hd.bottom+1})}})()""")
            assert r["n"] >= 3 and r["inHead"] and not r["floating"] and not r["overlap"] and r["row"], r
            assert r["names"][:2] == ["mapzin", "mapzout"] and "maplock" not in r["names"] and not r["explain"], r
            assert abs(r["h"][0] - r["ho"][0]) <= 1, r
        step("controls are in the panel head, none over the plot, no «How it is extracted»", place)
        def works():
            z = lambda: pg.evaluate(f"[{M}.zoom,{M}.zoomY]")
            pg.click(".pnl.map [data-nav=mapzin]"); settle(); zz = z(); assert zz[0] and zz[1], zz
            pg.click(".pnl.map [data-nav=mapzin]"); settle(); z2 = z(); assert z2[0][1] - z2[0][0] < zz[0][1] - zz[0][0], (zz, z2)
            pg.click(".pnl.map [data-nav=mapzout]"); settle(); z3 = z(); assert not z3[0] or z3[0][1] - z3[0][0] > z2[0][1] - z2[0][0], z3
            pg.click(".pnl.map [data-nav=mapzin]"); settle()
            pg.click(".pnl.map [data-a=fit]"); settle(); assert z() == [None, None], z()
            assert pg.locator(".pnl.map [data-nav=maplock]").count() == 0                  # no padlock button on the 2D map: the lock is in the menu and on L
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("l"); settle(); assert pg.evaluate(f"!!{M}._mz.lock")
            pg.keyboard.press("l"); settle(); assert not pg.evaluate(f"!!{M}._mz.lock")
            w0 = pg.evaluate(f"{M}.el.getBoundingClientRect().width")
            pg.click(".pnl.map [data-a=max]"); settle(); assert pg.evaluate(f"{M}.el.getBoundingClientRect().width") > w0 + 50
            pg.click(".pnl.map [data-a=max]"); settle()
        step("zoom, reset, lock (L) and full screen still work", works)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
