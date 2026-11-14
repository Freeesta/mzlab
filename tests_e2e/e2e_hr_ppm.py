"""B3: high-resolution spectra: own centroids, decimals of the profile, ruler in mDa, isotope pattern with ppm errors; QqQ and the 'off' switch unchanged."""
# covers: mzlab/web/hr.js, mzlab/reader/profile.py
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, real_cuttings, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_ppm.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrppm")
r = Run(port=8892, wd="/tmp/wdhr3")
J = lambda js: pg.evaluate(js)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        K = pg.evaluate("E.files.find(f=>f.file==='HR_DDA-Exploris-t30.mzML#MS1').k")
        def mk(k, rt, w=1e-4):
            return pg.evaluate(f"(()=>{{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');const s=newSpec(c,{rt}-{w},{rt}+{w},{k});return s.id}})()")
        def wait_draw(pid):
            pg.wait_for_function(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});return p&&p._a&&p._a.data&&p._a.data.length}})()", timeout=20000); pg.wait_for_timeout(300)
        rt16 = pg.evaluate(f"(async()=>{{const j=await (await fetch('api/dda?k={K}')).json();const i=j.ms1.rt.reduce((b,v,i)=>Math.abs(v-16)<Math.abs(j.ms1.rt[b]-16)?i:b,0);return j.ms1.rt[i]}})()")
        sp = mk(K, rt16); wait_draw(sp)
        def own_centroids():
            v = pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={sp});const mz=p._a.data[0].d.mz;return {{hrp:p._a.hrp,dec:p._a.dec,n:mz.length,a:mz.filter(m=>Math.abs(m-412.1)<0.002).length,b:mz.filter(m=>Math.abs(m-412.1364)<0.002).length,scans:p._a.data[0].d.scans}}}})()"); print(v)
            assert v["hrp"] and v["dec"] == 4 and v["a"] == 1 and v["b"] == 1 and v["scans"] == 1, v
        step("one scan at RT 16.0: 412.1000 and 412.1364 are two peaks, 4 decimals", own_centroids)
        def zoomed():
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={sp});p.zoom=[412.05,412.2];draw(p)}})()"); pg.wait_for_timeout(800)
            v = pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={sp}),a=p._a;const s1=a.snap(a.X(412.1364)),s0=a.snap(a.X(412.1000));p.meas={{ref:412.1000,list:[{{a:412.1000,b:412.1364}}]}};draw(p);const h=a.hov(a.X(412.1364));return {{s1:s1&&s1.m,s0:s0&&s0.m,lbl:a.lbls.map(l=>l.m),html:h&&h.html}}}})()"); print(v)
            assert abs(v["s1"] - 412.1364) < 1e-3 and abs(v["s0"] - 412.1) < 1e-3, v          # the nearest centroid, not an average
            assert __import__("re").search(r"m/z 412\.136\d", v["html"]), v
            assert any(abs(m - 412.1364) < 1e-3 for m in v["lbl"]) and any(abs(m - 412.1) < 1e-3 for m in v["lbl"]), v
        step("zoomed: snap, label and tooltip give the centroid with 4 decimals; ruler works below 0.04", zoomed)
        def isotopes():
            rt = 14.3; sp2 = mk(K, rt, 0.012); wait_draw(sp2)
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={sp2});p.iso={{formula:'C14H13F4N3O2S',ad:'[M+H]+'}};p.zoom=[362,370];draw(p)}})()"); pg.wait_for_timeout(900)
            t = pg.evaluate(f"E.panels.find(q=>q.id=={sp2}).leg2.textContent"); print(t)
            assert "profilo teorico" in t and "(M:" in t and "ppm" in t, t
            import re
            e = float(re.search(r"\(M: ([+−][0-9.]+) ppm", t).group(1).replace("−", "-")); assert abs(e) < 3, e          # 0.8 ppm noise
        step("isotope profile: the circle sits on the centroid and says the error in ppm", isotopes)
        def no_switch():
            assert pg.evaluate("UIP.hr === undefined && !HR.on") and not pg.evaluate("!!document.querySelector('#uip-hr')")
            v = pg.evaluate(f"(()=>{{const p=E.panels.find(x=>x.id=={sp});return [p._a.hrp,p._a.dec]}})()"); print(v)
            assert v[0] is True and v[1] == 4, v                        # an Orbitrap file is always read as high resolution
        step("no «Alta risoluzione» switch any more: the Exploris spectrum keeps its 4 decimals", no_switch)
        def qqq():
            pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500)
            load(pg, [D / "B_FullMass-t0.mzML"], 4000, 1)
            Q = pg.evaluate("E.files.find(f=>f.file==='B_FullMass-t0.mzML').k")
            q = mk(Q, 14.3, 0.02); wait_draw(q)
            v = pg.evaluate(f"(()=>{{const p=E.panels.find(x=>x.id=={q});return [p._a.hrp,p._a.dec]}})()"); print(v)
            assert v == [False, 1], v
        step("QqQ file (its own session): not high resolution, 1 decimal", qqq)
    r.close()
    cuts = real_cuttings()
    if cuts:                                                  # the real cuttings: a spectrum of one scan equals the scan stored in the file (generic property)
        r = Run(port=8893, wd="/tmp/wdhr4")
        with sync_playwright() as p:
            pg = r.page(p)
            load(pg, cuts[:1], 4000, 2, settle=6000)
            def real():
                K = pg.evaluate("E.files.find(f=>f.kind==='full').k")
                v = pg.evaluate(f"(async()=>{{const j=await (await fetch('api/dda?k={K}')).json(), n=j.ms1.sid.length>>1, rt=j.ms1.rt[n], sid=j.ms1.sid[n];const s=await (await fetch('api/scan?k={K}&sid='+sid)).json();const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');const sp=newSpec(c,rt-1e-4,rt+1e-4,{K});return {{id:sp.id,n:s.mz.length}}}})()")
                pg.wait_for_function(f"(()=>{{const p=E.panels.find(q=>q.id=={v['id']});return p&&p._a&&p._a.data&&p._a.data.length}})()", timeout=30000)
                got = pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={v['id']});return [p._a.hrp,p._a.dec,p._a.data[0].d.mz.length]}})()"); print(v, got)
                assert got[0] and got[1] == 4 and got[2] == v["n"], (v, got)
            step("real cutting: the spectrum of one scan has exactly the centroids of the scan", real)
        r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
