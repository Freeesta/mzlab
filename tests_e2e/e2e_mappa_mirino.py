"""Sight of the RT x m/z map (mirino.js + mappa.js), low and high resolution: the pointer moves fast and the page asks the engine little and only for
what the point needs (one request at a time, the newest point wins), a preview is drawn at once and the exact spectrum / XIC replace it, no exact
data of another point is ever drawn, a scan already visited is not asked again, and the final answer is the same as /api/nearest_scan + /api/spectrum + /api/xic."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, hr_variants
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_mirino.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
COUNT = ("(()=>{window.__rq=[];window.__fl=0;window.__flMax=0;const f=window.fetch;window.fetch=function(u,...a){const m=/api\\/([a-z_]+)/.exec(String(u));"
         "if(!m)return f.call(this,u,...a);window.__rq.push(m[1]);window.__fl++;window.__flMax=Math.max(window.__flMax,window.__fl);"
         "const d=()=>{window.__fl--};return f.call(this,u,...a).then(x=>{d();return x},e=>{d();throw e})}})()")
M = "E.panels.find(q=>q.type==='map')"
r = Run(port=8899, wd="/tmp/wd_mirino_e2e")
def case(pg, label):
    box = lambda: pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
    def prepare():
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); pg.wait_for_timeout(900); ready(pg)
        pg.evaluate(COUNT)
    step(f"{label}: full screen, sight on", prepare)
    c = box(); y0 = c[1] + c[3] * .5
    def sweep():
        pg.evaluate("window.__rq.length=0; window.__flMax=0")
        pg.mouse.move(c[0] + c[2] * .2, y0)
        samples = []
        for i in range(70):                                     # fast: no pause between events
            pg.mouse.move(c[0] + c[2] * .2 + i * 7, y0 + (i % 7) * 4)
            if i % 5 == 0: samples.append(pg.evaluate(f"(()=>{{const s={M}._mz.shown;return s?{{sp:s.sp,xi:s.xi,rt:s.pt.rt,mz:s.pt.mz}}:null}})()"))
        ready(pg); pg.wait_for_timeout(600)
        rq = pg.evaluate("window.__rq"); n = len(rq)
        assert "nearest_scan" not in rq and "spectrum" not in rq, rq                          # the scan is found in the page, one scan by position
        assert n <= 70 + 8, n                                                                 # at most about one request per event, never a queue per pixel
        assert pg.evaluate("window.__flMax") <= 2, pg.evaluate("window.__flMax")              # spectrum + XIC of one point; the newest wins
        ax = pg.evaluate("fetch('api/scan_axis?k=' + " + M + ".a_k).then(r=>r.json())") if False else None
        k = pg.evaluate(M + "._a.f.k"); lv = pg.evaluate(M + "._a.f.lv||1")
        axis = pg.evaluate(f"fetch('api/scan_axis?k={k}&level={lv}').then(r=>r.json())")
        near = lambda rt: min(range(len(axis["rt"])), key=lambda i: abs(axis["rt"][i] - rt))
        for s in samples:
            if s and s["sp"]: assert s["sp"].endswith("|%d" % axis["sid"][near(s["rt"])]), (s, "exact spectrum of another point")      # never a stale exact answer
        # the final state: exact spectrum and XIC of the final point
        sh = pg.evaluate(f"{M}._mz.shown"); assert sh["sp"] and sh["xi"], sh
        pt = sh["pt"]; i = near(pt["rt"]); assert sh["sp"].endswith("|%d" % axis["sid"][i]), (sh, axis["sid"][i])
        return rq
    step(f"{label}: 70 fast events: few requests, one at a time, newest wins, no stale exact data", sweep)
    def same():
        sh = pg.evaluate(f"({M}._mz.shown)"); pt = sh["pt"]
        n = pg.evaluate(f"fetch('api/nearest_scan?k={pg.evaluate(M + '._a.f.k')}&rt=' + {pt['rt']!r} + '&level={pg.evaluate(M + '._a.f.lv||1')}').then(r=>r.json())")
        old = pg.evaluate(f"fetch('api/spectrum?k={pg.evaluate(M + '._a.f.k')}&rt0=' + ({n['rt']} - 1e-4) + '&rt1=' + ({n['rt']} + 1e-4) + '&level={pg.evaluate(M + '._a.f.lv||1')}&bin=0.1' + MERGE()).then(r=>r.json())")
        sp = pg.evaluate(f"{M}._mz.spec"); assert sp["sid"] == n["sid"] and sp["mz"] == old["mz"] and sp["y"] == old["y"], (sp["sid"], n)
        xi = pg.evaluate(f"{M}._mz.xic"); k = pg.evaluate(M + "._a.f.k")
        ox = pg.evaluate(f"fetch('api/xic?k={k}&mz=' + {xi['c']!r} + '&tol=' + {xi['tol']!r} + '&level={pg.evaluate(M + '._a.f.lv||1')}').then(r=>r.json())")
        assert xi["tr"] == ox["traces"], "XIC differs from /api/xic"
    step(f"{label}: the final spectrum and XIC are those of nearest_scan + spectrum + xic", same)
    def preview():
        pg.evaluate("window.__seen=false;setInterval(()=>{const s=" + M + "._mz.shown;if(s&&(!s.sp||!s.xi))window.__seen=true},4)")
        pg.mouse.move(c[0] + c[2] * .8, y0 - 60); pg.mouse.move(c[0] + c[2] * .35, y0 + 50); pg.mouse.move(c[0] + c[2] * .65, y0 - 20)
        pg.wait_for_timeout(250); ready(pg); pg.wait_for_timeout(300)
        assert pg.evaluate("window.__seen"), "no preview was drawn before the exact answer"
        sh = pg.evaluate(f"{M}._mz.shown"); assert sh["sp"] and sh["xi"], sh                  # and the exact answer replaced it
    step(f"{label}: a preview (from the matrix of the map) first, the exact data replace it", preview)
    def cached():
        # 12 stops, each answered; the same 12 stops again ask nothing for the spectra (they come from the cache of the page)
        pos = [(c[0] + c[2] * (.25 + .04 * i), y0 + (i % 3) * 5) for i in range(12)]
        for q in pos: pg.mouse.move(*q); pg.wait_for_timeout(120); ready(pg)
        pg.wait_for_timeout(500); ready(pg)
        pg.evaluate("window.__rq.length=0")
        for q in pos: pg.mouse.move(*q); pg.wait_for_timeout(120); ready(pg)
        pg.wait_for_timeout(500)
        rq = pg.evaluate("window.__rq"); assert rq.count("spectra") == 0 and rq.count("xic") == 0, rq
    step(f"{label}: scans visited before come from the cache", cached)
    def keyboard():
        pg.evaluate(M + ".cv.focus()"); p0 = pg.evaluate(f"MAPPA.point({M})")
        pg.keyboard.press("ArrowRight"); pg.keyboard.press("ArrowRight"); ready(pg); pg.wait_for_timeout(400)
        p1 = pg.evaluate(f"MAPPA.point({M})"); assert p1["rt"] > p0["rt"]
        sh = pg.evaluate(f"{M}._mz.shown"); assert sh["sp"] and sh["xi"], sh
    step(f"{label}: arrows still move the sight scan by scan", keyboard)
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
        case(pg, "LR")
        # high resolution: the Full Scans of a synthetic Orbitrap file, in a session of their own
        D = synth("mirino"); hr = hr_variants(D / "HR_DDA-Exploris-t30.mzML", D)[0]
        pg2 = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg2.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg2.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        r2 = Run(port=8900, wd="/tmp/wd_mirino_e2e2")
        try:
            pg2.goto(f"http://127.0.0.1:{r2.port}/"); pg2.wait_for_timeout(800)
            pg2.set_input_files("#pick", [str(hr)]); pg2.wait_for_timeout(800)
            pg2.click("#opbtn"); ready(pg2); pg2.wait_for_timeout(1500); pg2.evaluate("document.querySelector('.fl')?.click()"); ready(pg2); pg2.wait_for_timeout(800)
            pg2.evaluate("reveal(addPanel('map',{k:E.files[0].k}))"); ready(pg2); pg2.wait_for_timeout(600)
            case(pg2, "HR")
        finally:
            r2.close()
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
