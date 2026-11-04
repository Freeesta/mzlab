"""H7: settings of the automatic integration: with the defaults the edges are exactly those of the algorithm that always was (values recorded before the change on four synthetic traces),
every setting of the dialog really changes the result, stored keys outside the dialog are dropped, and the dialog fits without scrolling at 1280x720 and 1366x768."""
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
EDGES = lambda extra="": f"(async()=>{{{SER} Object.assign(PK, PK_DEF); {extra}; return Promise.all(S.map(async s=>{{const e=await autoEdges(s,2.25);return e&&e.map(v=>+v.toFixed(5))}}))}})()"
BASE = [[2.15, 2.35], [2.125, 2.36], [1.93, 2.66], [2.2032, 2.295]]
r = Run(port=8974, wd="/tmp/wd_picchi")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.wait_for_timeout(1500)
        step("defaults reproduce the edges of before", lambda: (lambda v: (_ for _ in ()).throw(AssertionError(str(v))) if v != BASE else None)(pg.evaluate(EDGES())))
        def each():
            for extra in ("PK.edgeFrac=40", "PK.win=0.05", "PK.multi=300", "PK.tail=0.05"):
                v = pg.evaluate(EDGES(extra)); assert v != BASE, extra
            assert pg.evaluate(EDGES("PK.snMin=1e9")) == [None] * 4
        step("every setting changes the result; the S/N threshold refuses the peak", each)
        def dlg():
            pg.evaluate("Object.assign(PK, PK_DEF); peakParams()"); pg.wait_for_selector("#pk-ok", timeout=5000)
            assert pg.locator("#bigbody [data-pk]").count() == 7 and pg.locator("#bigbody fieldset.pk-sec").count() == 2
            pg.fill("[data-pk=edgeFrac]", "7"); pg.fill("[data-pk=hrSmooth]", "6"); pg.click("#pk-ok"); pg.wait_for_timeout(300)
            assert pg.evaluate("PK.edgeFrac") == 7 and pg.evaluate("PK.hrSmooth") == 7          # the smoothing window is odd
            assert json.loads(pg.evaluate("localStorage.getItem('mzlab.picchi')")) == {**pg.evaluate("PK_DEF"), "edgeFrac": 7, "hrSmooth": 7}
            pg.evaluate("peakParams()"); pg.click("#pk-def"); assert pg.evaluate("PK.edgeFrac") == 3 and pg.evaluate("PK.hrSmooth") == 5
            pg.evaluate("document.querySelector('#bigdlg').close()")
        step("the dialog sets, remembers and resets the settings", dlg)
        def dropped():
            pg.evaluate("localStorage.setItem('mzlab.picchi', JSON.stringify({areaNoise: 40, noise: 'rms', hrProm: 50, win: 0.8, tail: -1}))")
            pg.reload(); pg.wait_for_function("typeof PK === 'object'", timeout=20000)
            assert pg.evaluate("PK") == {**pg.evaluate("PK_DEF"), "win": 0.8}, pg.evaluate("PK")
            pg.evaluate("localStorage.removeItem('mzlab.picchi')")
        step("stored keys outside the dialog are dropped silently", dropped)
        def fits():
            for w, h in ((1280, 720), (1366, 768)):
                pg.set_viewport_size({"width": w, "height": h}); pg.evaluate("peakParams()"); pg.wait_for_selector("#pk-ok", timeout=5000)
                m = pg.evaluate("(()=>{const b=document.querySelector('#bigbody'),r=document.querySelector('#bigdlg').getBoundingClientRect();return [b.scrollHeight-b.clientHeight,b.scrollWidth-b.clientWidth,r.bottom<=innerHeight]})()")
                assert m[0] <= 1 and m[1] <= 1 and m[2], (w, h, m)
                pg.evaluate("document.querySelector('#bigdlg').close()")
        step("the dialog fits without scrolling at 1280x720 and 1366x768", fits)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
