"""DIA file in the DDA trio: its MS2 are isolation windows fragmented every cycle, not ions picked from the Full Scan. No flag claims a peak,
the empty MS2 panel says so, the «N MS²» list names each window («window m/z lo–hi»), a click on a peak opens the MS2 of the window that holds it
(its band over the Full Scan, no precursor line on the MS2), and the list closes on a pointerdown outside it (an iPad tap gives no mousedown)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
sys.path.insert(0, str(ROOT / "tools"))
from synthetic_data_series import dia
import numpy as np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_dda_dia.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
d = HERE / "shots" / "ddadia_tmp"; d.mkdir(parents=True, exist_ok=True)
dia(d / "D_DIA-t0.mzML", np.random.default_rng(67))
r = Run(port=8979, wd="/tmp/wdddadia")
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),c=s1&&E.panels.find(p=>p.id===s1.link),s2=s1&&E.panels.find(p=>p.id===s1.duo);return {c,s1,s2}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [d / "D_DIA-t0.mzML"], 6000, 1)
        wait_for_js(pg, "(()=>{const t=" + TRIO + ";return t.s1&&t.s1._a&&t.s1._a.data&&t.s2&&DDA.ready(t.s1.k)&&t.s1._a.markN!=null})()", 60000); ready(pg)
        def no_flags():
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const f=E.files[t.s1.k];return {acq:f.acq,dia:DDAMARK.dia(f),flags:t.s1._a.flags.length,markN:t.s1._a.markN,why:t.s1._a.ddaWhy,want:I18N.t('dda.dia.why')}})()"); print(v)
            assert v["dia"] and "DIA" in v["acq"], v
            assert v["flags"] == 0 and v["markN"] == 0 and v["why"] == v["want"], v
        step("a DIA Full Scan: no flag over a peak, the empty MS2 panel says the MS2 are windows", no_flags)
        def listing():
            wait_for_js(pg, "(()=>{const t=" + TRIO + ";const b=t.s1.el.querySelector('[data-a=\"ddaw\"]');return b&&b.getAttribute('aria-disabled')==='false'})()", 30000)
            pg.evaluate("(()=>{const t=" + TRIO + ";t.s1.el.querySelector('[data-a=\"ddaw\"]').click()})()"); ready(pg)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const o=[...t.s1.el.querySelectorAll('.dw-pop [role=option]')].map(x=>x.textContent);return {o,want:I18N.t('dda.win.dia1',{i:1,lo:'120.0',hi:'165.0',rt:'X'}).split(' · ')[0]}})()"); print(v)
            assert len(v["o"]) == 8 and v["want"] in [x.split(" · ")[0] for x in v["o"]], v
            box = pg.evaluate("(()=>{const r=document.querySelector('#flst').getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2}})()")
            pg.mouse.click(box["x"], box["y"]); ready(pg)
            assert pg.evaluate("(()=>{const t=" + TRIO + ";return !t.s1.el.querySelector('.dw-pop')})()"), "the list stays open after a click outside"
        step("the «N MS²» list names each isolation window and closes on a pointerdown outside", listing)
        def peak():
            at = pg.evaluate("""(()=>{const t=""" + TRIO + """;const a=t.s1._a,d=a.data[0].d,r=t.s1.cv.getBoundingClientRect();let jj=-1;
              for(let i=0;i<d.mz.length;i++) if(d.mz[i]>a.x0&&d.mz[i]<a.x1&&d.mz[i]>125&&d.mz[i]<475&&(jj<0||d.y[i]>d.y[jj])) jj=i;
              const X=v=>M.l+(v-a.x0)/(a.x1-a.x0)*(a.W-M.l-M.r),Y=v=>a.H-M.b-(v/a.ymax)*(a.H-M.t-M.b);
              return {m:d.mz[jj],x:r.left+X(d.mz[jj]),y:r.top+Math.max(M.t+4,Math.min(a.H-M.b-2,Y(d.y[jj])+3))}})()"""); print(at)
            pg.mouse.click(at["x"], at["y"]); ready(pg)
            wait_for_js(pg, "(()=>{const t=" + TRIO + ";return t.s2.sid!=null&&t.s2._scan})()", 30000)
            v = pg.evaluate("(m=>{const t=" + TRIO + ";const D=DDA.ready(t.s1.k),j=D.idx.get(t.s2.sid);return {lo:D.lo[j],hi:D.hi[j],m}})", at["m"]); print(v)
            assert v["lo"] <= v["m"] <= v["hi"], v
        step("a click on a peak opens the MS2 of the window that holds its m/z", peak)
finally:
    r.close()
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " -- " + s))
bad = [e for e in r.errs if e[0] in ("pageerror", "console.error")]
for e in bad: print("ERR", e)
sys.exit(1 if any(s != "ok" for _, s in steps) or bad else 0)
