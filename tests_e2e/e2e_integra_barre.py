"""Manual integration window: the area is the trace between the two bars cut vertically down to zero (no sloped baseline), the label and the area table show
the same number; the bar handles are 44 px targets that a finger/pen drags (and the arrows move); the cursor dot is hidden while the tool is on."""
# covers: mzlab/web/integra-zero.js, mzlab/web/integra-estremi.js, mzlab/web/barre-maniglie.js
import sys, os, math; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_barre.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
XP = "E.panels.find(p=>p.type==='xic')"
r = Run(port=8886, wd="/tmp/wd86")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_selector("text=Carica dati")
        pg.click("text=Carica dati"); ready(pg)
        def synthetic():                                               # a small peak on the tail of a big neighbour (8.13 and 8.5, 0.05 min sampling)
            xs = [round(7.5 + 0.01 * i, 4) for i in range(201)]
            ys = [1e6 * math.exp(-0.5 * ((x - 8.13) / 0.12) ** 2) + 2e5 * math.exp(-0.5 * ((x - 8.5) / 0.05) ** 2) for x in xs]
            a, b = 8.33, 8.71                                          # bars between samples: interpolation at the exact positions
            def at(t):
                j = max(i for i in range(len(xs)) if xs[i] <= t); f = (t - xs[j]) / (xs[j + 1] - xs[j]); return ys[j] + (ys[j + 1] - ys[j]) * f
            pts = [(a, at(a))] + [(x, y) for x, y in zip(xs, ys) if a < x < b] + [(b, at(b))]
            ref = sum((pts[i][1] + pts[i + 1][1]) / 2 * (pts[i + 1][0] - pts[i][0]) * 60 for i in range(len(pts) - 1))
            res = pg.evaluate("async ([xs,ys,a,b])=>{await carica('integra-zero.js');const s={x:xs,y:ys,ys};return {neu:integ(s,a,b,null,true).area,old:integ(s,a,b).area}}", [xs, ys, a, b])
            assert abs(res["neu"] - ref) < 1e-6 * ref, (res, ref)
            assert abs(res["old"] - ref) > 0.05 * ref, "the sloped baseline must give a clearly different area on this pair"
        step("overlapping Gaussians: new area = trapezoids with exact bar interpolation, old sloped area differs", synthetic)
        pg.evaluate("addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"); wait_for_js(pg, f"!!({XP}&&{XP}._a&&{XP}._a.sr&&{XP}._a.sr.length)")
        pg.evaluate(f"(()=>{{const p={XP};setActive(p);p.imode='man';p.intf=p._a.sr[0].key;p.el.scrollIntoView({{block:'center'}});cursorLine(p)}})()"); frames(pg, 3)
        def bars():
            c = pg.evaluate(f"(()=>{{const p={XP},a=p._a,q=p.cv.getBoundingClientRect();return {{l:q.left,t:q.top,a:a.X(13.93),b:a.X(14.57),y:a.H-M.b-6}}}})()")
            pg.mouse.move(c["l"] + c["a"], c["t"] + c["y"]); pg.mouse.down(); pg.mouse.move(c["l"] + c["b"], c["t"] + c["y"], steps=8); pg.mouse.up(); frames(pg, 3)
            assert pg.evaluate(f"{XP}.ints.length") == 1 and pg.evaluate(f"{XP}.ints[0].zero") is True
        step("the manual drag places the window with the zero baseline", bars)
        def number():
            it = pg.evaluate(f"(()=>{{const p={XP},i=p.ints[0],s=p._a.sr[0],xs=s.x,ys=s.y;const at=t=>{{let j=0;while(xs[j+1]<=t)j++;return ys[j]+(ys[j+1]-ys[j])*(t-xs[j])/(xs[j+1]-xs[j])}};const lo=Math.min(i.a,i.b),hi=Math.max(i.a,i.b),P=[[lo,at(lo)]];xs.forEach((x,j)=>{{if(x>lo&&x<hi)P.push([x,ys[j]])}});P.push([hi,at(hi)]);let ref=0;for(let j=1;j<P.length;j++)ref+=(P[j][1]+P[j-1][1])/2*(P[j][0]-P[j-1][0])*60;return {{area:i.area,ref,lab:p._a.lbls.find(l=>l.copy!=null).copy,txt:I18N.sciE(i.area)}}}})()")
            assert abs(it["area"] - it["ref"]) <= 1e-9 * max(1, it["ref"]) and abs(float(it["lab"]) - it["area"]) <= 1e-5 * it["area"] and it["txt"] in pg.evaluate(f"{XP}._a.lbls.find(l=>l.copy!=null).text"), it     # the label shows the page form and copies the CSV value (#3153)
            pg.evaluate("showInts()"); wait_for_js(pg, "document.querySelector('#bigbody table')&&document.querySelector('#bigbody table').textContent.length>0")
            assert it["txt"] in pg.evaluate("document.querySelector('#bigbody table').textContent") or it["txt"].replace(".", ",") in pg.evaluate("document.querySelector('#bigbody table').textContent"), it
            pg.keyboard.press("Escape")
        step("label (shown and copied) = trapezoid sum = number in the area table", number)
        def dot_hidden():
            assert pg.evaluate(f"{XP}.ch.hidden") is True, "the cursor dot must be hidden while the integration tool is on"
        step("cursor dot hidden in integration mode", dot_hidden)
        def targets():
            sz = pg.evaluate(f"[...{XP}.el.querySelectorAll('.ibh')].map(b=>{{const r=b.getBoundingClientRect();return [r.width,r.height,b.getAttribute('aria-label')]}})")
            assert len(sz) == 2 and all(w >= 44 and h >= 44 and lab for w, h, lab in sz), sz
        step("two bar handles of at least 44 px with an aria-label", targets)
        def touch_drag():
            pg.evaluate("document.documentElement.classList.add('ptr-touch');document.documentElement.classList.remove('ptr-mouse')")
            before = pg.evaluate(f"{XP}.ints[0].b")
            pg.evaluate(f"""(()=>{{const p={XP},b=p.el.querySelectorAll('.ibh')[1],q=b.getBoundingClientRect(),cx=q.left+q.width/2,cy=q.top+q.height/2,a=p._a,cv=p.cv.getBoundingClientRect();
              const ev=(t,x)=>b.dispatchEvent(new PointerEvent(t,{{pointerId:7,pointerType:'touch',isPrimary:true,clientX:x,clientY:cy,bubbles:true,cancelable:true}}));
              ev('pointerdown',cx);ev('pointermove',cv.left+a.X({before}+0.1));ev('pointerup',cv.left+a.X({before}+0.1))}})()"""); frames(pg, 3)
            after = pg.evaluate(f"{XP}.ints[0].b"); assert after > before + 0.05, (before, after)
            assert pg.evaluate(f"{XP}.ints[0].zero") is True
        step("a touch drag on the right handle moves the bar", touch_drag)
        def keys():
            before = pg.evaluate(f"{XP}.ints[0].b"); pg.evaluate(f"{XP}.el.querySelectorAll('.ibh')[1].focus()"); pg.keyboard.press("ArrowLeft"); frames(pg, 3)
            assert pg.evaluate(f"{XP}.ints[0].b") < before
        step("arrow keys move the focused bar", keys)
        pg.screenshot(path="tests_e2e/shots/integra_barre.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
