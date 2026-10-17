"""B5: DDA trio: chromatogram, Full Scan (MS1) and MS2 one under the other, flags on the precursors, triangles, arrows in the MS2 panel."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_dda.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrdda")
r = Run(port=8895, wd="/tmp/wdhr6")
TRIO = "(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),s1=E.panels.find(p=>p.type==='spec'&&p.link===c.id),s2=E.panels.find(p=>p.dda!=null);return {c,s1,s2}})()"
GEO = "(()=>{const t=" + TRIO + ";const g=p=>p&&[p.x,p.y,p.w,p.h];return [g(t.c),g(t.s1),g(t.s2)]})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2)
        pg.wait_for_timeout(2500)
        def layout():
            c, s1, s2 = pg.evaluate(GEO); print(c, s1, s2)
            assert s2 is not None, "no MS2 panel"
            assert s1[1] == c[1] + c[3], (c, s1)                           # Full Scan touches the chromatogram
            assert s1[0] == 0 and abs(s1[2] - c[2]) <= 2, (c, s1)          # MS1 has the whole width
            assert s2[0] == 0 and abs(s2[2] - c[2]) <= 2 and s2[1] >= s1[1] + s1[3], (c, s1, s2)       # MS2 under it, as wide
            assert pg.evaluate("document.querySelector('#dpanels').scrollHeight") > 0
        step("trio: c on top, MS1 under it at full width, MS2 under the MS1", layout)
        def narrow():
            pg.set_viewport_size({"width": 900, "height": 2200}); pg.wait_for_timeout(1500)
            c, s1, s2 = pg.evaluate(GEO); assert s2[1] >= s1[1] + s1[3] and s2[0] == 0, (s1, s2)           # under the Full Scan
            pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(1500)
            c, s1, s2 = pg.evaluate(GEO); assert s2[1] >= s1[1] + s1[3] and s2[0] == 0, (s1, s2)
        step("narrow or wide, the MS2 stays under the Full Scan", narrow)
        def tri():
            pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');if(!c.ms2tri){c.ms2tri=true;draw(c)}})()"); pg.wait_for_timeout(1000)
            t = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');c.el.scrollIntoView({block:'center'});const tr=c._a.tri.filter(q=>q.rt>13.5&&q.rt<15)[0];const r=c.cv.getBoundingClientRect();return {x:r.left+tr.px,y:r.top+M.t+4,sid:tr.sid,rt:tr.rt,n:c._a.tri.length}})()"); print(t)
            assert t["n"] > 50 and t["sid"] is not None
            pg.mouse.click(t["x"], t["y"]); pg.wait_for_timeout(2500)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {sid:t.s2.sid,scan:t.s2._scan&&t.s2._scan.sid,parent:t.s2._scan&&t.s2._scan.parent,s1sid:t.s1._a&&t.s1._a.data[0].d.sid,title:t.s2.title,cap:t.s2.leg.textContent,cur:t.c.cur}})()"); print(v)
            assert v["sid"] == t["sid"] == v["scan"], v
            assert v["s1sid"] == v["parent"], v                              # s1 shows exactly the parent Full Scan
            assert v["title"].startswith("MS2 di ") and "HCD" in v["cap"] and "NCE 30" in v["cap"] and "isolamento ±0.75" in v["cap"] and "madre" in v["cap"], v
            assert "eV" not in v["cap"], v                                    # Thermo: never eV
        step("a triangle on the chromatogram: s2 shows that MS2, s1 its parent Full Scan", tri)
        def flags():
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {n:t.s1._a.flags.length,lbl:t.s1._a.lbls.filter(l=>l.fl!=null).length,tip:t.s1._a.lbls.find(l=>l.fl!=null).tip}})()"); print(v)
            assert v["n"] >= 1 and v["lbl"] == v["n"] and "MS2 n. " in v["tip"] and "HCD" in v["tip"] and "NCE 30" in v["tip"], v
        step("flags ▼ over the precursors of the shown Full Scan, with hover text", flags)
        def flag_click():
            # a Full Scan with two MS2 of close precursors (412.1000 and 412.1364): choose the first, then click the second flag
            two = pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);for(const [par,kids] of D.byParent){{const k=kids.filter(i=>Math.abs(D.prec[i]-D.prec[kids[0]])<1);if(k.length>=2)return {par,first:D.sid[k[0]],second:D.sid[k[1]],rt:D.prt[k[0]]}}}return null})()"); print(two)
            assert two, "no Full Scan with 2 MS2"
            pg.evaluate("(async()=>{const t=" + TRIO + ";await DDA.selectMs2(t.s2," + str(two["first"]) + ")})()"); pg.wait_for_timeout(2000)
            pos = pg.evaluate("(()=>{const t=" + TRIO + ";t.s1.el.scrollIntoView({block:'center'});const l=t.s1._a.lbls.filter(l=>l.fl!=null);const r=t.s1.cv.getBoundingClientRect();const D=DDA.ready(t.s1.k);const q=l.find(b=>D.sid[b.fl]==" + str(two["second"]) + ");return q&&{x:r.left+q.x+q.w/2,y:r.top+q.y+q.h/2}})()"); print(pos)
            assert pos, "second flag not visible"
            pg.mouse.click(pos["x"], pos["y"]); pg.wait_for_timeout(2000)
            sid = pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2.sid})()"); assert sid == two["second"], (sid, two)
        step("click on a flag: s2 shows that MS2", flag_click)
        def arrows():
            before = pg.evaluate("(()=>{const t=" + TRIO + ";setActive(t.s2);return t.s2.sid})()")
            pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(1500)
            after = pg.evaluate("(()=>{const t=" + TRIO + ";return [t.s2.sid,t.s1._a.data[0].d.sid,t.s2._scan.parent]})()"); print(before, after)
            assert after[0] > before and after[1] == after[2], (before, after)
            pg.keyboard.press("ArrowLeft"); pg.wait_for_timeout(1500)
            back = pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2.sid})()"); assert back == before, (before, back)
        step("arrows in the MS2 panel: next / previous MS2; s1 follows the parent", arrows)
        def close_and_back():
            n0 = pg.evaluate("E.panels.length")
            pg.evaluate("(()=>{const t=" + TRIO + ";t.s2.el.querySelector('.x').click()})()"); pg.wait_for_timeout(800)
            assert pg.evaluate("E.panels.find(p=>p.dda!=null)") is None and pg.evaluate("E.panels.length") == n0 - 1
            geo = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full'),s1=E.panels.find(p=>p.link===c.id);return [c.y+c.h,s1.y,s1.w,hostWidth()]})()"); print(geo)
            assert geo[2] > geo[3] - 5, geo                                      # the Full Scan has the whole width again
            pos = pg.evaluate("(()=>{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');c.el.scrollIntoView({block:'center'});const r=c.cv.getBoundingClientRect();return {x:r.left+300,y:r.top+150}})()")
            pg.mouse.click(pos["x"], pos["y"], button="right"); pg.wait_for_timeout(400)
            assert "Mostra le MS2 sotto la Full Scan" in pg.inner_text("#ctx")
            pg.click("#ctx >> text=Mostra le MS2 sotto la Full Scan"); pg.wait_for_timeout(2500)
            c, s1, s2 = pg.evaluate(GEO); print(c, s1, s2)
            assert s2 is not None and s2[1] >= s1[1] + s1[3] and s2[0] == 0, (s1, s2)
        step("close the MS2 panel, then «Mostra le MS2 sotto la Full Scan» in the right-click menu brings it back", close_and_back)
        def reload():
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);await DDA.selectMs2(t.s2,D.sid[7])})()"); pg.wait_for_timeout(1500)
            saved(pg); pg.reload(); attendi(pg, "typeof E !== 'undefined' && E.files.length > 0 && E.panels.length > 0", 90000); ready(pg)
            c, s1, s2 = pg.evaluate(GEO); sid = pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2.sid})()"); print(c, s1, s2, sid)
            assert s2 and s1[1] == c[1] + c[3] and s2[1] >= s1[1] + s1[3] and sid is not None
        step("after a reload the trio and the chosen MS2 are back", reload)
    r.close()
    r = Run(port=8896, wd="/tmp/wdhr7")
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Fusion-t30.mzML"], 6000, 2); pg.wait_for_timeout(2500)
        def fusion():
            t = pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);await DDA.selectMs2(t.s2,D.sid[3]);return D.sid[3]})()"); pg.wait_for_timeout(2000)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {cap:t.s2.leg.textContent,mz:t.s2._a.data[0].d.mz.slice(0,6),dec:t.s2._a.dec,title:t.s2.title}})()"); print(v)
            assert "CID" in v["cap"] and "NCE 35" in v["cap"] and "isolamento ±1.5" in v["cap"] and v["dec"] == 2, v
            assert all(len(str(m).split(".")[1] if "." in str(m) else "") <= 3 for m in v["mz"]) and v["title"].startswith("MS2 di ") and len(v["title"].split(".")[1]) == 2, v
        step("Fusion (ion trap MS2): 2 decimals, CID, NCE 35, isolation ±1.5", fusion)
    r.close()
    r = Run(port=8897, wd="/tmp/wdhr8")
    ida = D / "C_IDA-t0.mzML"; ds.mixed_ida(ida, np.random.default_rng(1))
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [ida], 6000, 2); pg.wait_for_timeout(2500)
        def qqq_ida():
            t = pg.evaluate("(async()=>{const t=" + TRIO + ";if(!t.s2)return null;const D=await DDA.get(t.s1.k);await DDA.selectMs2(t.s2,D.sid[5]);return D.sid[5]})()"); pg.wait_for_timeout(2000)
            assert t is not None, "the IDA file of the QTRAP has no trio"
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {cap:t.s2.leg.textContent,hrp:t.s2._a.hrp,dec:t.s2._a.dec}})()"); print(v)
            assert "CE 25 eV" in v["cap"] and "NCE" not in v["cap"] and v["hrp"] is False and v["dec"] == 1, v
        step("QTRAP IDA file: the trio too, low resolution, CE in eV", qqq_ida)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
