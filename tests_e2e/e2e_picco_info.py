"""Peak info on a spectrum card (peak-info.js): hover on a small unlabelled peak, on a labelled one and far from any peak (MS2 and MS3, synthetic data),
the loss from the precursor, no box while dragging, the arrow keys with the live region, and the click that pins / removes a label."""
# covers: mzlab/web/peak-info.js
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
from dati_sintetici_mzml import _write
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_picco_info.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n"," | ")[:400]))
D = Path(tempfile.mkdtemp(prefix="picco_info_")); F = D / "S_MSn.mzML"
# MS1 + MS2 of 283.1 (212.2, 265.3, 283.2 tall; four small peaks under the 5 % label threshold) + MS3 of 212.2
MS2 = ([150.1, 190.0, 212.2, 230.4, 245.6, 265.3, 283.2], [0.02, 0.03, 1.0, 0.015, 0.025, 0.4, 0.7])
MS3 = ([95.0, 123.0, 141.1, 160.5, 171.0], [0.03, 0.5, 1.0, 0.02, 0.2])
specs = []
for i in range(6):
    rt = 9.4 + 0.1 * i
    specs.append(ds._spec(3 * i, rt, np.array([283.1, 300.2]), np.array([1e6, 2e5]), level=1))
    specs.append(ds._spec(3 * i + 1, rt + 0.02, np.array(MS2[0]), np.array(MS2[1]) * 1e5, level=2, prec=283.1, ce=20.0))
    specs.append(ds._spec(3 * i + 2, rt + 0.04, np.array(MS3[0]), np.array(MS3[1]) * 1e4, level=3, prec=212.2, ce=30.0))
rts = [9.4 + 0.1 * i for i in range(6)]
_write(F, specs, [ds._chrom(0, "TIC", rts, [1e6] * 6), ds._chrom(1, "BPC", rts, [1e6] * 6)])
S2 = "E.panels.find(p=>p.type==='spec'&&p.tab==='ms2')"
def where(pg, m, frac=0.0):
    """page coordinates of the stick of m/z m (frac of its height from the top) and of the baseline"""
    return pg.evaluate(f"(()=>{{const p={S2},a=p._a,b=p.cv.getBoundingClientRect(),d=a.data[0].d,j=d.mz.findIndex(v=>Math.abs(v-{m})<0.01);return [b.left+a.X(d.mz[j]),b.top+a.Y(d.y[j])+(a.Y(0)-a.Y(d.y[j]))*{frac},b.top+a.Y(0)-2]}})()")
def tip(pg): return pg.evaluate(f"(()=>{{const t={S2}.tip;return t.hidden?'':t.innerText}})()")
r = Run(port=8881, wd="/tmp/wd81")
try:
    with sync_playwright() as p:
        shot = os.environ.get("PICCO_SHOT")
        pg = r.page(p, **({"device_scale_factor": 2} if shot else {}))
        stage(pg, [str(F)]); pg.click("text=Load data"); ready(pg); open_kind(pg, "ms2")
        wait_for_js(pg, f"(()=>{{const p={S2};return !!p&&!!p._a&&p._a.data[0].d.mz.length>0}})()")
        pg.evaluate(f"{S2}.el.scrollIntoView({{block:'center'}})"); frames(pg, 10)
        def small():
            assert pg.evaluate(f"!{S2}._a.lbls.some(l=>l.m!=null&&Math.abs(l.m-150.1)<0.01)"), "150.1 must be unlabelled"
            x, y, _ = where(pg, 150.1, 0.5); pg.mouse.move(x, y); frames(pg, 3)
            t = tip(pg); assert "150.1" in t and "intensity" in t and "2000" in t and "2.0 %" in t, t
            assert "from precursor 283.1" in t and "−133.0" in t and "loss 133.0" in t, t
        step("MS2: hover on a small unlabelled peak shows m/z, intensity (value and %) and the loss", small)
        def rel():
            pg.evaluate(f"(()=>{{const p={S2};p.rel=true;draw(p)}})()"); frames(pg, 4)
            x, y, _ = where(pg, 150.1, 0.5); pg.mouse.move(x, y); frames(pg, 3)
            t = tip(pg); assert "2.0 %" in t and "(2000 cps)" in t and "−133.0" in t, t
            pg.evaluate(f"(()=>{{const p={S2};p.rel=false;draw(p)}})()"); frames(pg, 3)
        step("in % mode the box adds the absolute value", rel)
        def stick():
            assert pg.evaluate(f"(()=>{{const e={S2}._pkhi;return !!e&&!e.hidden&&e.offsetHeight>=2}})()")
        step("the stick of the peak is highlighted", stick)
        def labelled():
            x, y, _ = where(pg, 212.2, 0.5); pg.mouse.move(x, y); frames(pg, 3)
            t = tip(pg); assert "212.2" in t and "100.0 %" in t and "from precursor 283.1" in t and "−70.9" in t, t
        step("MS2: hover on a labelled peak shows the same box", labelled)
        def far():
            x, y, base = where(pg, 150.1); pg.mouse.move(x + 120, base - 30); frames(pg, 3)
            assert tip(pg) == "" and pg.evaluate(f"{S2}._pkhi.hidden"), tip(pg)
        step("far from every peak: no box", far)
        def drag():
            x, y, base = where(pg, 150.1, 0.5); pg.mouse.move(x, y); frames(pg, 2); pg.mouse.down(); pg.mouse.move(x + 60, y + 4, steps=4); frames(pg, 3)
            assert tip(pg) == "", tip(pg)
            pg.mouse.up(); frames(pg, 2); pg.evaluate(f"{S2}.zoom=null;draw({S2})"); frames(pg, 3)
        step("no box while dragging", drag)
        def keys():
            pg.evaluate(f"{S2}.cv.focus()"); pg.keyboard.press("ArrowRight"); frames(pg, 3)
            t = tip(pg); assert "150.1" in t, t
            pg.keyboard.press("ArrowRight"); frames(pg, 3); assert "190.0" in tip(pg), tip(pg)
            pg.keyboard.press("ArrowLeft"); frames(pg, 3); assert "150.1" in tip(pg), tip(pg)
            pg.keyboard.press("Shift+ArrowRight"); frames(pg, 3); assert "245.6" in tip(pg), tip(pg)      # the next taller peak than 150.1 (2.0 %): 245.6 (2.5 %)
            pg.keyboard.press("Shift+ArrowLeft"); frames(pg, 3); assert "150.1" in tip(pg), tip(pg)
            live = pg.evaluate(f"{S2}._pklive.textContent"); assert "150.1" in live and "283.1" in live, live
            pg.keyboard.press("Escape"); frames(pg, 2); assert tip(pg) == ""
        step("arrow keys walk the peaks, Shift by intensity, the live region says it, Esc closes", keys)
        def pin():
            x, y, _ = where(pg, 150.1, 0.5); pg.mouse.click(x, y); frames(pg, 4)
            assert pg.evaluate(f"{S2}._a.lbls.some(l=>l.m!=null&&Math.abs(l.m-150.1)<0.01)"), "pinned"
            assert pg.evaluate(f"JSON.stringify({S2}.lblOn)") != "undefined"
            x, y, _ = where(pg, 212.2, 0.5); pg.mouse.click(x, y); frames(pg, 4)
            assert pg.evaluate(f"!{S2}._a.lbls.some(l=>l.m!=null&&Math.abs(l.m-212.2)<0.01)"), "removed"
            pg.evaluate(f"COMMANDS.undo ? COMMANDS.undo() : 0")
        step("a click pins the label of a small peak and removes the one of a labelled peak", pin)
        def msn3():
            pg.evaluate(f"(()=>{{const p={S2};p.level=3;p.prec=212.2;p.lblOn=[];p.lblOff=[];draw(p)}})()"); frames(pg, 6)
            wait_for_js(pg, f"{S2}._a&&{S2}._a.data[0].d.mz.length>0&&Math.abs({S2}._a.data[0].d.mz[0]-95.0)<0.1")
            x, y, _ = where(pg, 95.0, 0.5); pg.mouse.move(x, y); frames(pg, 3)
            t = tip(pg); assert "95.0" in t and "from precursor 212.2" in t and "−117.2" in t, t
        step("MS3: hover on a small peak gives the loss from its precursor", msn3)
        errs = [e for e in r.errs if "iframe" not in str(e)]
        print("--- BROWSER ERRORS", len(errs)); [print(e) for e in errs[:5]]
        if shot:
            pg.evaluate(f"(()=>{{const p={S2};p.level=2;p.prec=283.1;p.lblOn=[];p.lblOff=[];draw(p)}})()"); frames(pg, 6)
            x, y, _ = where(pg, 150.1, 0.5); pg.mouse.move(x, y); frames(pg, 3)
            bb = pg.evaluate(f"(()=>{{const b={S2}.el.getBoundingClientRect();return {{x:b.left,y:b.top,width:b.width,height:b.height}}}})()"); pg.screenshot(path=shot, clip=bb)
finally: r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(1 if bad else 0)
