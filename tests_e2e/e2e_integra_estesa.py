"""An integration whose baseline line (`base` of an HR automatic window, still valid after an edge is dragged) is extended past its two points by dragging the
edge is held at zero there (integra-base.js, mzlab_core::baseline `at`): the area is measured by the engine against that line, never counts a triangle under
the axis, the polygon drawn is the number, and the label's hover text says what was not counted. Inside its two points the record is unchanged (integ())."""
# covers: mzlab/web/integra-base.js
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_estesa.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "integra_estesa_tmp"; D.mkdir(parents=True, exist_ok=True)
HRF = D / ds.HR_NAMES["exploris"]
_g0 = ds._g                       # every ion elutes twice: a copy at 70 % 0.3 min later, so one XIC trace holds two peaks that overlap
ds._g = lambda rt, c, s, tau=0.04: _g0(rt, c, s, tau) + 0.7 * _g0(rt, c + 0.3, s, tau)
try: ds.hr_dda(HRF, np.random.default_rng(31), "exploris")
finally: ds._g = _g0
PN = "E.panels.find(q=>q.type==='xic')"
BASE = "[S.x[nearIdx(S.x,9.14)],3e5,S.x[nearIdx(S.x,9.4)],0]"      # at sample times, as the engine writes them: 3e5 counts at the valley falling to 0 at the end of the second peak; past it the line alone is negative
OUT = Path(os.environ.get("ESTESA_SHOTS", str(D)))

def record(pg, b):
    """one `edges` record with that baseline between 9.14 and 9.40, the right edge then set to `b`; waits for the engine's number when it has to ask"""
    pg.evaluate(f"""(()=>{{const p={PN},s=p._a.sr[0];p.ints=[];const S=s,i=intRec(p,s,S.x[nearIdx(S.x,9.14)],S.x[nearIdx(S.x,9.4)],null,false);Object.assign(i,{{bm:'edges',base:{BASE}}});delete i.zero;p.ints.push(i);i.b={b};draw(p)}})()""")
    wait_for_js(pg, f"(()=>{{const p={PN},i=p.ints[0],s=p._a.sr[0];if(!Number.isFinite(i.area))return false;if(!INTBASE.dips(s,i))return i.clamped===0||i.clamped===undefined;return i.clamped>0}})()")

def engine(pg):
    return json.loads(pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i=p.ints[0];return QUALITA.peakArea(s.x,s.y,i.a,i.b,2,i.base)}})()"))

POLY = f"""(()=>{{const p={PN},s=p._a.sr[0],i=p.ints[0],paths=[];let cur=[];
  const g=new Proxy({{}},{{get:(_,k)=>k==='createPattern'?(()=>null):k==='moveTo'||k==='lineTo'?((x,y)=>{{if(k==='moveTo')cur=[];cur.push([x,y])}}):k==='fill'?(()=>paths.push(cur.slice())):(()=>{{}}),set:()=>true}});
  INTBASE.paint(g,t=>t,v=>v,(q,v)=>v,s,i,false);return paths[0]||[]}})()"""

r = Run(port=8996, wd="/tmp/wd_integra_estesa")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [HRF], 4000, 1)
        pg.evaluate("openXic(null)"); pg.wait_for_selector("#xic-mz"); pg.fill("#xic-mz", "305.0702"); pg.select_option("#xic-file", index=1); pg.click("#xic-go"); ready(pg)
        wait_for_js(pg, f"(()=>{{const p={PN};return !!(p&&p._a&&p._a.sr&&p._a.sr.length)}})()")
        pg.evaluate(f"(()=>{{zoomTo({PN},8.4,10.2)}})()"); wait_for_js(pg, f"Math.abs({PN}._a.x1-10.2)<0.05")

        def inside():
            record(pg, 9.4)
            i = pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i=p.ints[0];return {{area:i.area,clamped:i.clamped,dips:INTBASE.dips(s,i),j:integ(s,i.a,i.b,i.base,false).area}}}})()")
            assert not i["dips"] and not i["clamped"] and i["area"] == i["j"], i          # the old number, from integ(), byte for byte
            e = engine(pg); assert abs(e["area"] - i["area"]) <= 1e-9 * i["area"] and "clamped" not in e, (e, i)      # and the engine's line gives the same
            pg.locator(".pnl.xic").first.screenshot(path=str(OUT / "dentro.png"))
        step("inside its two points the record is the one of before (integ(), no clamp)", inside)

        def dragged():
            record(pg, 9.75)
            i = pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i=p.ints[0];return {{area:i.area,below:i.below,clamped:i.clamped,dips:INTBASE.dips(s,i),raw:integ(s,i.a,i.b,i.base,false).area,x:s.x.filter(t=>t>=i.a&&t<=i.b).map(t=>INTBASE.edgeAt(s,i,t))}}}})()")
            assert i["dips"] and i["clamped"] > 0 and i["area"] >= 0, i
            assert min(i["x"]) >= 0, i                                      # no point of the line is under the axis
            e = engine(pg); assert abs(e["area"] - i["area"]) <= 1e-9 * i["area"] and abs(e["clamped"] - i["clamped"]) <= 1e-9 * i["clamped"], (e, i)
            assert i["raw"] > i["area"], i                                  # the old number counted the triangle under the axis
            assert abs((i["raw"] - i["area"]) - i["clamped"]) <= 1e-6 * i["raw"], i      # and exactly that is what the record says it did not count
            pg.locator(".pnl.xic").first.screenshot(path=str(OUT / "estesa.png"))
        step("dragged past the end the line is held at 0: area >= 0, the engine's number, the part not counted is stated", dragged)

        def polygon():
            v = pg.evaluate(POLY)
            assert len(v) > 6, v
            assert min(y for _, y in v) >= 0, [q for q in v if q[1] < 0][:3]        # no vertex under the clamp
            a = abs(sum(v[k][0] * v[(k + 1) % len(v)][1] - v[(k + 1) % len(v)][0] * v[k][1] for k in range(len(v)))) / 2 * 60
            e = json.loads(pg.evaluate(f"(()=>{{const p={PN},s=p._a.sr[0],i=p.ints[0];return QUALITA.peakArea(s.x,s.ys,i.a,i.b,2,i.base)}})()"))["area"]      # the curve as drawn (ys)
            assert abs(a - e) <= 1e-6 * e, (a, e)                                   # the polygon drawn is the engine's number for it
        step("the polygon drawn has no vertex under zero and its area is the number", polygon)

        def hover():
            ok = pg.evaluate(f"(()=>{{const p={PN},i=p.ints[0];return (p._a.lbls||[]).filter(l=>l.int===i).every(l=>!l.tip)}})()")
            assert ok, "the area label has no hover text"
        step("the area label has no hover text, even where the baseline was held at 0", hover)
        pg.context.close() if False else None
        r.b.close()
finally:
    r.srv.terminate()
for n, s in steps: print(f"{n}: {s}")
if any(s != "ok" for _, s in steps): raise SystemExit(1)
