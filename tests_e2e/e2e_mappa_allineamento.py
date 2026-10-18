"""Map RT x m/z alignment and header robustness (PKG_MAPPA_INTESTAZIONE_PDA):
- Header remains on a single line (< 36px) even with a 60-character file name;
- Side spectrum (m/z) and bottom XIC (RT) precisely align with the map canvas (tolerance <= 1px);
- Window resize / full-screen maintains exact alignment."""
import sys, os
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

EX = ROOT / "mzlab" / "web" / "esempi"
M = "E.panels.find(q=>q.type==='map')"
steps = []
r = Run(port=8909, wd="/tmp/wd_mapali_e2e")

def step(name, fn):
    try:
        fn()
        steps.append((name, "ok"))
    except Exception as e:
        steps.append((name, "FAIL " + repr(e)[:600]))

try:
    with sync_playwright() as p:
        b = p.chromium.launch()
        r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML")]); pg.wait_for_timeout(800)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg); pg.wait_for_timeout(400)
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})"); pg.wait_for_timeout(300)

        # Activate sight
        pg.click(".pnl.map [data-mz=mir]"); pg.wait_for_timeout(400)
        box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        at = lambda fx, fy: (lambda c: (c[0] + c[2] * fx, c[1] + c[3] * fy))(box())

        # Move mouse over map center to set sight point
        x, y = at(.5, .5)
        pg.mouse.move(x, y); pg.mouse.move(x + 2, y + 2)
        pg.wait_for_function(f"!!({M}._mz.spec && {M}._mz.xic)", timeout=15000); pg.wait_for_timeout(300)

        def check_alignment():
            return pg.evaluate(f"""(() => {{
                const p = {M}, s = p._mz, a = p._a, pt = MAPPA.point(p);
                if (!s || !s.sp || !s.xc || !a || !pt) return null;
                const cr = p.cv.getBoundingClientRect();
                const sr = s.sp.getBoundingClientRect();
                const xr = s.xc.getBoundingClientRect();
                const hd = p.el.querySelector('.hd').getBoundingClientRect();
                const rd = p.el.querySelector('.rd');

                // Map crosshair in viewport coords
                const map_y = cr.top + a.Y(pt.mz);
                const map_x = cr.left + a.X(pt.rt);

                // Side spectrum guide line in viewport coords
                const mapTop = (cr.top - sr.top) + M.t;
                const mapBot = (cr.bottom - sr.top) - M.b;
                const mapH = mapBot - mapTop;
                const sp_line_y = sr.top + (mapBot - (pt.mz - a.y0) / (a.y1 - a.y0) * mapH);

                // Bottom XIC guide line in viewport coords
                const ml = (cr.left - xr.left) + M.l;
                const mr = (cr.right - xr.left) - M.r;
                const pw = mr - ml;
                const xc_line_x = xr.left + (ml + (pt.rt - a.x0) / (a.x1 - a.x0) * pw);

                return {{
                    hd_h: hd.height,
                    has_rd: !!rd,
                    diff_y: Math.abs(map_y - sp_line_y),
                    diff_x: Math.abs(map_x - xc_line_x)
                }};
            }})()""")

        def step_long_name():
            long_name = "X" * 60
            pg.evaluate(f"(()=>{{ E.files[0].label = '{long_name}'; E.files[0].name = '{long_name}'; ctl({M}); draw({M}); }})()")
            pg.wait_for_timeout(500)
            x, y = at(.5, .5)
            pg.mouse.move(x, y); pg.mouse.move(x + 2, y + 2)
            pg.wait_for_timeout(300)
            res = check_alignment()
            assert res is not None, "alignment data not available"
            assert not res["has_rd"], "2D map must not have .rd element in header"
            assert res["hd_h"] < 36, f"header wrapped to multi-line: height={res['hd_h']}px"
            assert res["diff_y"] <= 1.0, f"m/z alignment error: {res['diff_y']}px"
            assert res["diff_x"] <= 1.0, f"RT alignment error: {res['diff_x']}px"
        step("Passo 3a: 60-char filename + sight: header stays 1 row (<36px) and m/z & RT aligned (<=1px)", step_long_name)

        def step_resize():
            # Resize viewport to 1280px (narrower)
            pg.set_viewport_size({"width": 1280, "height": 850})
            pg.wait_for_timeout(600)
            x, y = at(.5, .5)
            pg.mouse.move(x, y); pg.mouse.move(x + 2, y + 2)
            pg.wait_for_timeout(300)
            res = check_alignment()
            assert res is not None, "alignment data not available after resize"
            assert res["diff_y"] <= 1.0, f"m/z alignment error on resize: {res['diff_y']}px"
            assert res["diff_x"] <= 1.0, f"RT alignment error on resize: {res['diff_x']}px"

            # Fullscreen toggle
            pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f")
            pg.wait_for_timeout(700)
            x, y = at(.5, .5)
            pg.mouse.move(x, y); pg.mouse.move(x + 2, y + 2)
            pg.wait_for_timeout(300)
            res_fs = check_alignment()
            assert res_fs is not None, "alignment data not available in fullscreen"
            assert res_fs["diff_y"] <= 1.0, f"m/z alignment error in fullscreen: {res_fs['diff_y']}px"
            assert res_fs["diff_x"] <= 1.0, f"RT alignment error in fullscreen: {res_fs['diff_x']}px"
            pg.keyboard.press("f"); pg.wait_for_timeout(500)
        step("Passo 3b: viewport resize and full screen: alignment preserved (<=1px)", step_resize)

        r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    r.close()

for s in steps:
    print(" ", s)
r.report()
