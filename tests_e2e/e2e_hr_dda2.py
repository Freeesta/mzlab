"""B6: DDA complete: nearby precursors, average of the MS2 of an ion, follow an ion, ions without MS2, co-isolation."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_dda2.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrdda2")
r = Run(port=8898, wd="/tmp/wdhr9")
TRIO = "(()=>{const s1=E.panels.find(p=>p.type==='spec'&&p.duo!=null),c=E.panels.find(p=>p.id===s1.link),s2=E.panels.find(p=>p.dda!=null);return {c,s1,s2}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 6000, 2); pg.wait_for_timeout(2500)
        def show_scan(rt):                       # the Full Scan nearest to rt (a window of +-1e-4 min holds exactly one scan)
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);const r=D.ms1.rt;let b=0;r.forEach((v,i)=>{if(Math.abs(v-" + str(rt) + ")<Math.abs(r[b]-" + str(rt) + "))b=i});const x=r[b];t.c.cur=x;t.s1.zoom=null;draw(t.c);pushLinked(t.c,x-1e-4,x+1e-4,t.s1.k)})()"); pg.wait_for_timeout(2500)
        def right_click(sel_js):
            pos = pg.evaluate("(()=>{const t=" + TRIO + ";t.s1.el.scrollIntoView({block:'center'});const r=t.s1.cv.getBoundingClientRect();const q=" + sel_js + ";return q&&{x:r.left+q.x+q.w/2,y:r.top+q.y+q.h/2}})()")
            assert pos, "target not found"
            pg.mouse.click(pos["x"], pos["y"], button="right"); pg.wait_for_timeout(400)
        def nearby():
            # an MS2 of the parent ion, and a Full Scan 0.15 min later that did not trigger it
            v = pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);const j=D.sid.findIndex((s,i)=>Math.abs(D.prec[i]-364.0737)<0.002&&D.rt[i]>14.0);return {rt:D.rt[j],prec:D.prec[j]}})()"); print(v)
            show_scan(v["rt"] + 0.15)
            q = pg.evaluate("(()=>{const t=" + TRIO + ";return {nb:t.s1._a.lbls.filter(l=>l.nb).map(l=>[l.ion,l.nb.length]),fl:t.s1._a.lbls.filter(l=>l.fl!=null).length,sid:t.s1._a.data[0].d.sid}})()"); print(q)
            hit = [n for n in q["nb"] if abs(n[0] - v["prec"]) < 0.002]; assert hit and hit[0][1] >= 1, q
            sid0 = q["sid"]
            right = "t.s1._a.lbls.find(l=>l.nb&&Math.abs(l.ion-" + str(v["prec"]) + ")<0.002)"
            pos = pg.evaluate("(()=>{const t=" + TRIO + ";const r=t.s1.cv.getBoundingClientRect();const q=" + right + ";return {x:r.left+q.x+q.w/2,y:r.top+q.y+q.h/2}})()")
            pg.mouse.click(pos["x"], pos["y"]); pg.wait_for_timeout(2000)
            w = pg.evaluate("(()=>{const t=" + TRIO + ";return {ion:t.s2.ion,sid:t.s2.sid,s1sid:t.s1._a.data[0].d.sid,scan:t.s2._scan&&t.s2._scan.prec}})()"); print(w)
            assert abs(w["ion"] - v["prec"]) < 0.002 and w["sid"] is not None and w["s1sid"] == sid0, w          # the Full Scan stays where it is
        step("▽ on a peak fragmented in another scan; a click shows the closest MS2 and sets the ion", nearby)
        def average():
            show_scan(pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2._scan.rt})()") + 0.15)
            right_click("t.s1._a.lbls.find(l=>l.nb&&Math.abs(l.ion-364.0737)<0.002)")
            t = pg.inner_text("#ctx"); print(t)
            assert "Media delle MS2 di questo ione" in t and "Segui questo ione nelle MS2" in t, t
            pg.click("#ctx >> text=Media delle MS2 di questo ione"); pg.wait_for_timeout(2500)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {n:t.s2.avg&&t.s2.avg.length,title:t.s2.title,cap:t.s2.leg.textContent,pk:t.s2._a.data[0].d.mz.length}})()"); print(v)
            assert v["n"] and 2 <= v["n"] <= 5 and v["title"].startswith("Media di ") and "364.073" in v["title"] and "media di " in v["cap"], v
        step("right click: average of the MS2 of that ion (at most 5, within ±1 min)", average)
        def follow():
            show_scan(9.0)
            right_click("(()=>{const a=t.s1._a;const s=a.data[0].d;let j=0;s.mz.forEach((m,i)=>{if(Math.abs(m-305.0702)<0.003)j=i});const X=a.X(s.mz[j]);return {x:X-4,y:20,w:8,h:a.H-60}})()")
            pg.wait_for_timeout(200)
            t = pg.inner_text("#ctx"); print(t.replace("\n", " | ")[:300])
            assert "Segui questo ione nelle MS2" in t, t
            pg.click("#ctx >> text=Segui questo ione nelle MS2"); pg.wait_for_timeout(4000)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const x=E.panels.find(p=>p.id===t.s1.link),c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');return {type:x.type,follow:!!x.ddaFollow,ion:t.s2.ion,sid:t.s2.sid,tri:x._a.tri.map(q=>q.prec),y:[c.y,x.y,t.s1.y],h:[c.h,x.h],ms2tri:[c.ms2tri,x.ms2tri],prec:t.s2._scan&&t.s2._scan.prec,title:x.title}})()"); print(v)
            assert v["type"] == "xic" and v["follow"] and abs(v["ion"] - 305.0702) < 0.003 and v["sid"] is not None, v
            assert v["tri"] and all(abs(q - 305.0702) < 0.003 for q in v["tri"]), v                  # only the triangles of that ion
            assert v["y"][1] == v["y"][0] + v["h"][0] + 10 and v["y"][2] == v["y"][1] + v["h"][1], v      # XIC under the TIC, the Full Scan touches the XIC
            assert abs(v["prec"] - 305.0702) < 0.003 and v["ms2tri"] == [False, True], v
        step("follow an ion: XIC under the chromatogram, Full Scan / MS2 follow it, only that ion's triangles, its most intense MS2 shown", follow)
        def arrows_ion():
            ids = pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);return DDA.list(t.s2,D).map(i=>D.sid[i])})()")
            pg.evaluate("(()=>{const t=" + TRIO + ";setActive(t.s2)})()")
            cur = pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2.sid})()"); i = ids.index(cur); print(ids, cur)
            if i + 1 < len(ids):
                pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(1500)
                assert pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2.sid})()") == ids[i + 1]            # next MS2 of the SAME ion
            else:
                pg.keyboard.press("ArrowLeft"); pg.wait_for_timeout(1500)
                assert pg.evaluate("(()=>{const t=" + TRIO + ";return t.s2.sid})()") == ids[i - 1]
        step("arrows in the MS2 panel walk through the MS2 of the followed ion", arrows_ion)
        def never():
            pg.evaluate("(async()=>{const t=" + TRIO + ";await DDA.follow(t.s1,250.1234)})()"); pg.wait_for_timeout(2500)
            w = pg.evaluate("(()=>{const t=" + TRIO + ";return [DDA.waiting(t.s2),t.s2.sid]})()"); print(w)
            assert w[0] == "Nessuna MS2 di 250.1234: il DDA non l'ha scelto." and w[1] is None, w
        step("an ion never fragmented: «Nessuna MS2 di 250.1234: il DDA non l'ha scelto»", never)
        def coiso():
            pg.evaluate("(async()=>{const t=" + TRIO + ";await DDA.follow(t.s1,229.05)})()"); pg.wait_for_timeout(2500)
            w = pg.evaluate("(()=>{const t=" + TRIO + ";return DDA.waiting(t.s2)})()"); print(w)
            assert "Nessuna MS2 di 229.0500" in w and "finestra di isolamento di " in w and "spettri misti" in w, w
            pg.evaluate("(()=>{const t=" + TRIO + ";setActive(t.s2)})()"); pg.keyboard.press("ArrowRight"); pg.wait_for_timeout(2500)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";return {sid:t.s2.sid,prec:t.s2._scan&&t.s2._scan.prec,lo:t.s2._scan&&t.s2._scan.lo,hi:t.s2._scan&&t.s2._scan.hi}})()"); print(v)
            assert v["sid"] is not None and v["lo"] <= 229.05 <= v["hi"] and abs(v["prec"] - 229.60) < 0.02, v        # the strong neighbour 229.60 co-isolated 229.05
        step("co-isolation: 229.05 is inside the window of the MS2 of 229.60; the arrows scroll them", coiso)
        def band():
            pg.evaluate("(async()=>{const t=" + TRIO + ";const D=await DDA.get(t.s1.k);const i=D.sid.findIndex((s,i)=>Math.abs(D.prec[i]-229.6)<0.02);await DDA.selectMs2(t.s2,D.sid[i])})()"); pg.wait_for_timeout(2500)
            v = pg.evaluate("(()=>{const t=" + TRIO + ";const D=DDA.ready(t.s1.k);const i=D.idx.get(t.s2.sid);return {parent:D.parent[i]===t.s1._a.data[0].d.sid,zoom:t.s1.zoom,lo:D.lo[i],hi:D.hi[i]}})()"); print(v)
            assert v["parent"] and v["lo"] < 229.05 < 229.6 < v["hi"], v                                  # the window of 229.60 contains 229.05
            assert v["zoom"] and abs((v["zoom"][0] + v["zoom"][1]) / 2 - (v["lo"] + v["hi"]) / 2) < 0.2, v    # the Full Scan went to the precursor ±5
        step("isolation band: the window of the MS2 of 229.60 holds 229.05; the Full Scan zooms to the precursor", band)
        def plist():
            pg.evaluate("setTab('ms2',true)"); pg.wait_for_timeout(2500)
            drawer(pg)                                 # the list button lives in the file drawer
            assert pg.is_visible("#ddalist"), "no button of the list"
            pg.click("#ddalist"); pg.wait_for_timeout(1500)
            rows = pg.evaluate("[...document.querySelectorAll('#ddatb tr[data-r]')].map(r=>[r.children[0].textContent,r.children[1].textContent,r.children[2].textContent])"); print(rows[:4], len(rows))
            assert len(rows) >= 10 and all(len(x[0].split(".")[1]) == 4 for x in rows), rows           # the precursors, 4 decimals
            pg.click("#ddatb th[data-c=n]"); pg.wait_for_timeout(300)
            ns = [int(x) for x in pg.evaluate("[...document.querySelectorAll('#ddatb tr[data-r] td:nth-child(3)')].map(t=>t.textContent)")]; assert ns == sorted(ns), ns
            pg.click("#ddatb tr[data-r]:has-text('346.12')")
            PROBE = "(()=>{const t=" + TRIO + ";return {tab:E.tab,ion:t.s2.ion,sid:t.s2.sid,prec:t.s2._scan&&t.s2._scan.prec}})()"
            for _ in range(40):                          # the MS2 of the row is read when ready (the time depends on the machine)
                pg.wait_for_timeout(500); v = pg.evaluate(PROBE)
                if v["sid"] is not None and v["prec"] and abs(v["prec"] - 346.1218) < 0.003: break
            print(v)
            assert v["tab"] == "full" and abs(v["ion"] - 346.1218) < 0.003 and v["sid"] is not None and abs(v["prec"] - 346.1218) < 0.003, v
        step("MS2 tab of a DDA file: list of the precursors (4 decimals, sortable); a row shows its most intense MS2 next to the Full Scan", plist)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
