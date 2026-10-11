"""H6: the isotope simulation of a high-resolution spectrum takes the resolving power into account: the fine structure of M+2 is ONE peak at R 27 000 (m/z 319) and separates at high R;
the M+1/M ratio of the formula; the menu entry sets R."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_isotopi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hriso")
r = Run(port=8972, wd="/tmp/wd_hriso")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1400})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        PAT = "(()=>{const i=QQQRef.ionCounts('C13H24N4O3S','[M+H]+');return HR.isoFine(i.n,i.z)})()"
        def counts():
            n = lambda R, lo, hi: pg.evaluate(f"HR.mergeRes({PAT},{R}).filter(r=>r.mz>{lo}&&r.mz<{hi}).length")
            fine = pg.evaluate(f"{PAT}.filter(r=>r.mz>318.6&&r.mz<319.4).length")
            assert fine >= 3, fine                                    # M+2 has 34S, 13C2, 18O (and more): several exact masses
            a = n(35000, 318.6, 319.4); assert a == 1, a             # R200 35000 -> about 27 700 at m/z 319: one peak
            b = n(300000, 318.6, 319.4); assert b >= 3, b            # at R 240 000 and more they separate
            assert pg.evaluate("HR.mergeRes(" + PAT + ", 0).length") == pg.evaluate(PAT + ".length")      # R = 0: whole fine structure
        step("M+2 of C13H25N4O3S: one peak at R 27 000 (m/z 319), separated at high resolution", counts)
        def ratio():
            v = pg.evaluate(f"(()=>{{const m=HR.mergeRes({PAT},35000);return m[1].rel/m[0].rel}})()")
            assert abs(v - 0.167) < 0.01, v
        step("M+1/M of the simulated ion is 0.167", ratio)
        def menu():
            sp = pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q._a&&q._a.hrp);return s?s.id:null})()"); assert sp is not None
            pg.evaluate(f"(()=>{{const s=E.panels.find(q=>q.id=={sp});s.iso={{formula:'C13H24N4O3S',ad:'[M+H]+'}};s.zoom=[314,325];s._isoKey=JSON.stringify(s.zoom);draw(s)}})()"); pg.wait_for_timeout(500)
            pg.evaluate(f"E.panels.find(q=>q.id=={sp}).el.scrollIntoView({{block:'center'}})"); pg.wait_for_timeout(300)
            leg = pg.evaluate(f"E.panels.find(q=>q.id=={sp}).el.innerText"); assert "profilo teorico" in leg and "R " in leg, leg[-300:]
        step("the legend of the simulation says the resolving power used", menu)
        def curve():
            v = pg.evaluate("(()=>{const i=QQQRef.ionCounts('C13H24N4O3S','[M+H]+');const pat=HR.mergeBy(HR.isoFine(i.n,i.z),m=>0.01);const c=HR.isoCurve(pat,m=>0.01,10);const mx=Math.max(...c.y);const k=c.y.indexOf(mx);return {mx,mz:c.x[k],n:c.x.length,top:pat.reduce((a,b)=>b.rel>a.rel?b:a).mz,w10:HR.WDEF['10']}})()")
            assert abs(v["mx"] - 100) < 1e-6 and abs(v["mz"] - v["top"]) < 0.005 and v["n"] > 100, v
            assert abs(v["w10"] - 1.8227) < 1e-3
        step("Gaussian profile: maximum 100 on the most intense peak", curve)
        def ui():
            pg.evaluate("BARRA.setTab('iso')"); pg.wait_for_timeout(500)
            pg.fill("#hri-f", "C13H24N4O3S"); pg.select_option("#hri-st", "profilo"); pg.fill("#hri-r", "60000")
            n0 = pg.evaluate("E.panels.length")
            pg.click("#hri-new"); pg.wait_for_function(f"E.panels.length=={n0}+1", timeout=15000); pg.wait_for_timeout(800)
            q = pg.evaluate("(()=>{const s=E.panels[E.panels.length-1];return {t:s.type,st:s.iso.style,R:s.iso.R,only:s.iso.only}})()")
            assert q == {"t": "spec", "st": "profilo", "R": 60000, "only": False}, q
            pg.click("#hri-rep"); pg.wait_for_timeout(1200)
            q = pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q.iso&&q.iso.only);return s?s.id:null})()"); assert q is not None
            pg.select_option("#hri-rm", "ppm"); pg.fill("#hri-r", "5"); pg.click("#hri-go"); pg.wait_for_timeout(800)
            fw = pg.evaluate("(()=>{const s=E.panels.find(q=>q.type==='spec'&&q.iso&&!q.iso.only&&q.iso.fw);return s&&s.iso.fw})()"); assert fw == {"mode": "ppm", "v": 5}, fw
        step("tab Isotopi: Nuova cella, Sostituisci, width in ppm", ui)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
