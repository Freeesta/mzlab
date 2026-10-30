"""One click of the automatic integration tool on a high-resolution XIC integrates every peak of the stretch shown (Rust auto_integrate, peakq_auto.rs),
in one undoable step; on a low-resolution XIC the same click still integrates only the peak under the cursor, with the edges of autoEdges."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_hr.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "integra_hr_tmp"; D.mkdir(parents=True, exist_ok=True)
HRF, LRF = D / ds.HR_NAMES["exploris"], D / "B_FullMass-t30.mzML"
_g0 = ds._g                       # every ion of the HR file elutes twice: a copy at 60 % 2.5 min later, so one XIC trace holds two peaks
ds._g = lambda rt, c, s, tau=0.04: _g0(rt, c, s, tau) + 0.6 * _g0(rt, c + 2.5, s, tau)
try: ds.hr_dda(HRF, np.random.default_rng(31), "exploris")
finally: ds._g = _g0
ds.full_scan(LRF, 30, np.random.default_rng(2026))
WINS = "(()=>{const p=E.panels.find(q=>q.type==='xic');return p.ints.map(i=>[+i.a.toFixed(3),+i.b.toFixed(3)]).sort((u,v)=>u[0]-v[0])})()"

def xic(pg, mz):
    pg.evaluate("openXic(null)"); pg.wait_for_selector("#xic-mz"); pg.fill("#xic-mz", mz); pg.select_option("#xic-file", index=1); pg.click("#xic-go"); ready(pg)
    attendi(pg, "(()=>{const p=E.panels.find(q=>q.type==='xic');return !!(p&&p._a&&p._a.sr&&p._a.sr.length)})()")

def click_at(pg, rt):
    v = pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.type==='xic'),r=p.cv.getBoundingClientRect();return {{x:r.left+p._a.X({rt}),y:r.top+r.height*0.5}}}})()")
    pg.mouse.click(v["x"], v["y"])

for kind in ("hr", "lr"):
    r = Run(port=8981 if kind == "hr" else 8982, wd=f"/tmp/wd_integra_{kind}")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            load(pg, [HRF if kind == "hr" else LRF], 4000, 1)
            if kind == "hr":
                xic(pg, "305.0702")          # 5 ppm: the ion at RT 9.0 and its copy at 11.5
                def all_peaks():
                    assert pg.evaluate("hrTrace(E.panels.find(q=>q.type==='xic')._a.sr[0])")
                    pg.locator(".pnl.xic [data-a=iauto]").first.click(); click_at(pg, 10.2)
                    attendi(pg, "E.panels.find(q=>q.type==='xic').ints.length>=2")
                    w = pg.evaluate(WINS); assert len(w) == 2, w
                    assert w[0][0] < 9.0 < w[0][1] and w[1][0] < 11.5 < w[1][1] and w[0][1] <= w[1][0], w
                step("one click on an empty stretch integrates both peaks of the HR trace", all_peaks)
                def parity():
                    v = pg.evaluate("""(async()=>{const p=E.panels.find(q=>q.type==='xic'),s=p._a.sr[0],a=p._a;
                      const r=await QUALITA.autoIntegrate(s.x,s.y,a.x0,a.x1,{smooth:PK.hrSmooth,snr:PK.hrSn,rel:PK.hrProm/100,scans:PK.hrScans,half:PK.hrHalf,slope:PK.hrSlope/100});
                      const it=[...p.ints].sort((u,w)=>u.a-w.a);return it.map((i,k)=>[i.area,r[k].area*60,!!i.base,integ(s,i.a,i.b).area])})()""")
                    for page, rust, has_base, chord in v:
                        assert has_base and abs(page - rust) <= 1e-6 * rust, v       # the page area over the Rust baseline is the Rust area
                        assert chord > 0, v
                step("the area shown is the one of the Rust engine, over its own baseline", parity)
                def undo_redo():
                    pg.evaluate("STORIA.annulla()"); attendi(pg, "E.panels.find(q=>q.type==='xic').ints.length===0")
                    click_at(pg, 10.2); attendi(pg, "E.panels.find(q=>q.type==='xic').ints.length===2")
                    click_at(pg, 10.2); attendi(pg, "!document.querySelector('#qnote').hidden")
                    assert pg.evaluate("E.panels.find(q=>q.type==='xic').ints.length") == 2          # a second click adds nothing over the peaks already integrated
                step("one undo removes all of them; a second click adds no overlapping window", undo_redo)
                def zoomed():
                    pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='xic');p.ints=[];zoomTo(p,8,10.5)})()"); attendi(pg, "Math.abs(E.panels.find(q=>q.type==='xic')._a.x1-10.5)<0.3")
                    click_at(pg, 9.5); attendi(pg, "E.panels.find(q=>q.type==='xic').ints.length>=1")
                    w = pg.evaluate(WINS); assert len(w) == 1 and w[0][0] < 9.0 < w[0][1], w
                step("zoomed in, only the peaks of the stretch shown", zoomed)
                def params():
                    pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='xic');p.ints=[];zoomTo(p,0,20);PK.hrProm=80})()"); attendi(pg, "E.panels.find(q=>q.type==='xic')._a.x1>15")
                    click_at(pg, 10.2); attendi(pg, "E.panels.find(q=>q.type==='xic').ints.length>=1")
                    w = pg.evaluate(WINS); assert len(w) == 1 and w[0][0] < 9.0 < w[0][1], w             # the copy stands at 60 % of the tallest: under 80 %
                    pg.evaluate("Object.assign(PK, PK_DEF)")
                step("the minimum height parameter leaves out the smaller peak", params)
            else:
                xic(pg, "364")
                def lr_one():
                    assert not pg.evaluate("hrTrace(E.panels.find(q=>q.type==='xic')._a.sr[0])")
                    pg.locator(".pnl.xic [data-a=iauto]").first.click(); click_at(pg, 14.3)
                    attendi(pg, "E.panels.find(q=>q.type==='xic').ints.length>=1")
                    w = pg.evaluate(WINS); assert len(w) == 1, w
                    ed = pg.evaluate("(()=>{const p=E.panels.find(q=>q.type==='xic'),i=p.ints[0];const e=autoEdges(p._a.sr[0],14.3);return [Math.abs(e[0]-i.a),Math.abs(e[1]-i.b)]})()")
                    assert max(ed) < 0.02, ed
                    click_at(pg, 5.0)
                    assert pg.evaluate("E.panels.find(q=>q.type==='xic').ints.length") <= 2          # one click = at most one peak, as always
                step("LR: one click integrates one peak with the edges of autoEdges", lr_one)
            assert_no_miss = pg.evaluate("(window.MZLAB_LR_MISS||[]).length"); assert assert_no_miss == 0, assert_no_miss
        r.close()
    except Exception as e:
        steps.append((f"run {kind}", "FAIL " + str(e)[:300]))
        try: r.close()
        except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
