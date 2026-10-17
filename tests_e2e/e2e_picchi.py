"""H7: parameters of the automatic integration: with the defaults the edges are exactly those of the algorithm that always was (values recorded before the change on four synthetic traces),
and every parameter really changes the result."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_picchi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
SER = """const mk=(n,dt,f)=>{const x=[],y=[];for(let i=0;i<n;i++){x.push(i*dt);y.push(f(i*dt,i))}return {x,y}};
 let seed=7; const rnd=()=>{seed=(seed*16807)%2147483647;return seed/2147483647-0.5};
 const g=(t,c,s,h)=>h*Math.exp(-0.5*((t-c)/s)**2);
 const S=[mk(900,0.005,(t)=>g(t,2.25,0.05,1000)+200*rnd()+50), mk(900,0.005,(t)=>g(t,2.25,0.05,1000)+g(t,2.45,0.05,600)+60*rnd()),
  mk(900,0.005,(t)=>g(t,2.25,0.12,5000)+g(t,2.6,0.04,300)+30*rnd()+100), mk(3000,0.0017,(t)=>g(t,2.25,0.02,400)+40*rnd())];"""
EDGES = lambda extra="": f"(()=>{{{SER} Object.assign(PK, PK_DEF); {extra}; return S.map(s=>{{const e=autoEdges(s,2.25);return e&&e.map(v=>+v.toFixed(5))}})}})()"
BASE = [[2.15, 2.35], [2.125, 2.36], [1.93, 2.66], [2.2032, 2.295]]
r = Run(port=8974, wd="/tmp/wd_picchi")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.wait_for_timeout(1500)
        step("defaults reproduce the edges of before", lambda: (lambda v: (_ for _ in ()).throw(AssertionError(str(v))) if v != BASE else None)(pg.evaluate(EDGES())))
        def each():
            for extra in ("PK.areaNoise=40", "PK.edgeFrac=40", "PK.noise='rms'", "PK.win=0.05", "PK.minHalf=12", "PK.multi=300", "PK.tail=0.05"):
                v = pg.evaluate(EDGES(extra)); assert v != BASE, extra
            assert pg.evaluate(EDGES("PK.snMin=1e9")) == [None] * 4
        step("every parameter changes the result; the S/N threshold refuses the peak", each)
        def constr():
            v = pg.evaluate("(()=>{"+SER+" Object.assign(PK,PK_DEF); PK.constrain=20; const e=autoEdges(S[2],2.25,0.2); return e[1]-e[0]})()"); assert 0.159 < v < 0.241, v
            assert pg.evaluate("(()=>{"+SER+" Object.assign(PK,PK_DEF); PK.constrain=20; const e=autoEdges(S[2],2.25,0); return +(e[1]-e[0]).toFixed(5)})()") == 0.73       # no reference peak: free
        step("«Vincola la larghezza»: the base stays within ±20% of the reference peak", constr)
        def man():
            b = pg.evaluate(EDGES("PK.noise='manual'; PK.nr0=0; PK.nr1=0"))
            assert b == BASE, b                                       # manual without a region: the automatic noise
            v = pg.evaluate(EDGES("PK.noise='manual'; PK.nr0=1.9; PK.nr1=2.6")); assert v != BASE, v
        step("manual noise region", man)
        def dlg():
            pg.evaluate("Object.assign(PK, PK_DEF); peakParams()"); pg.wait_for_selector("#pk-ok", timeout=5000)
            pg.fill("[data-pk=areaNoise]", "7"); pg.select_option("[data-pk=noise]", "rms"); pg.click("#pk-ok"); pg.wait_for_timeout(300)
            assert pg.evaluate("PK.areaNoise") == 7 and pg.evaluate("PK.noise") == "rms"
            assert json.loads(pg.evaluate("localStorage.getItem('mzlab.picchi')"))["areaNoise"] == 7
            pg.evaluate("peakParams()"); pg.click("#pk-def"); assert pg.evaluate("PK.areaNoise") == 3 and pg.evaluate("PK.noise") == "mad"
        step("the dialog sets, remembers and resets the parameters", dlg)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
