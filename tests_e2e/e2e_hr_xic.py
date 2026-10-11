"""B4: XIC with the tolerance of the profile (5 ppm for the Orbitrap), the unit window for the QqQ, and the switch «spenta»."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_hr_xic.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrxic")
r = Run(port=8894, wd="/tmp/wdhr5")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        K = pg.evaluate("E.files.find(f=>f.file==='HR_DDA-Exploris-t30.mzML#MS1').k")
        T0 = None                                                 # the ion trace of the first XIC, kept for the low-resolution session at the end
        def via_dialog(txt):
            pg.evaluate("openXic(null,{})"); pg.wait_for_timeout(400)
            pg.fill("#xic-mz", txt); pg.wait_for_timeout(400)
            summ = pg.inner_text("#xic-sum")
            pg.click("#xic-go"); ready(pg)
            return summ, pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().id")
        def series(pid):
            pg.wait_for_function(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});return p&&p._a&&p._a.sr&&p._a.sr.length}})()", timeout=20000)
            return pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});return p._a.sr.map(s=>{{const n=s.y.length;let b=0;s.y.forEach((v,i)=>{{if(v>s.y[b])b=i}});const at=t=>{{let j=0;s.x.forEach((v,i)=>{{if(Math.abs(v-t)<Math.abs(s.x[j]-t))j=i}});return Math.max(...s.y.slice(Math.max(0,j-2),j+3))}};return {{k:s.k,rt:s.x[b],max:s.y[b],at9:at(9.0),at12:at(12.0)}}}})}})()")
        def isobars():
            sm, pid = via_dialog("305.0702"); print(sm)
            assert "ppm" in sm and "305.0702" in sm, sm               # high resolution window only (a QqQ file cannot be open together)
            tr = pg.evaluate(f"E.panels.find(q=>q.id=={pid}).traces[0]"); assert tr["ion"] and abs(tr["mz"] - 305.0702) < 1e-6 and "± 5 ppm" in tr["label"], tr
            s = {x["k"]: x for x in series(pid)}; print(s)
            ex = s[K]; assert abs(ex["rt"] - 9.0) < 0.15 and ex["at9"] > 1e6 and ex["at12"] < 1e4, ex           # the isobar at 12.0 min is not in the window
            sm2, pid2 = via_dialog("305.1066")
            s2 = {x["k"]: x for x in series(pid2)}[K]; print(s2)
            assert abs(s2["rt"] - 12.0) < 0.15 and s2["at12"] > 1e6 and s2["at9"] < 1e4, s2
        step("XIC 305.0702 gives the peak at 9.0 min only; 305.1066 the peak at 12.0 min only", isobars)
        def hr_args():
            global T0
            pid = pg.evaluate("E.panels.filter(p=>p.type==='xic')[0].id")
            T0 = pg.evaluate(f"E.panels.find(q=>q.id=={pid}).traces[0]")
            a = pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid}),t=p.traces[0];return HR.xicArgs(E.files[{K}],t,p)}})()"); print(a)
            assert abs(a[1] - 305.0702 * 5e-6) < 1e-7, a                      # Orbitrap: 5 ppm
        step("the Orbitrap file uses its ppm tolerance", hr_args)
        def direct():
            pid = pg.evaluate("(()=>{const p=xicDirect(412.1364);return p.id})()"); pg.wait_for_timeout(1500)
            tr = pg.evaluate(f"E.panels.find(q=>q.id=={pid}).traces[0]"); print(tr)
            assert tr["ion"] and tr["label"] == "m/z 412.1364 ± 5 ppm", tr
            assert pg.evaluate(f"xicName(E.panels.find(q=>q.id=={pid}))") == "m/z 412.1364 ± 5 ppm"
        step("click on a centroid: XIC centred on it, named «m/z 412.1364 ± 5 ppm»", direct)
        def edit_edges():
            pid = pg.evaluate("E.panels.filter(p=>p.type==='xic').pop().id")
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});ctl(p)}})()")
            assert pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});return [p.el.querySelectorAll('[data-o=xlo],[data-o=xhi]').length,p.el.querySelector('.xlab').textContent]}})()") == [0, "XIC \u2022 m/z 412.1364"], "HR header: only «XIC • m/z value»"
            # m/z and interval live in «Display options» of that chromatogram
            def item(i): return f"DISP.items.chrom.find(x=>x.id==='{i}')"
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});{item('xtol')}.set(p,'3')}})()"); pg.wait_for_timeout(500)
            t = pg.evaluate(f"E.panels.find(q=>q.id=={pid}).traces[0]"); assert t["ion"] and t["ppm"] == 3 and t["label"].endswith("\u00b1 3 ppm"), t
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});{item('xunit')}.set(p,'da')}})()"); pg.wait_for_timeout(500)
            t = pg.evaluate(f"E.panels.find(q=>q.id=={pid}).traces[0]"); assert abs(t["wda"] - 412.1364 * 3e-6) < 2e-5 and "ppm" not in t["label"], t
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={pid});{item('xmz')}.set(p,'412.1370')}})()"); pg.wait_for_timeout(500)
            t = pg.evaluate(f"E.panels.find(q=>q.id=={pid}).traces[0]"); assert abs(t["mz"] - 412.137) < 1e-6, t
            assert pg.evaluate(f"E.panels.find(q=>q.id=={pid}).el.querySelector('.xlab').textContent") == "XIC \u2022 m/z 412.1370"
        step("HR XIC: header «XIC • m/z value»; m/z and interval (ppm / Da) in Display options", edit_edges)
        def no_switch():
            assert pg.evaluate("UIP.hr === undefined && !HR.on") and not pg.evaluate("!!document.querySelector('#uip-hr')")
            pid = pg.evaluate("E.panels.filter(p=>p.type==='xic')[0].id")
            s = {x["k"]: x for x in series(pid)}[K]; print(s)
            assert s["at9"] < 1e4 or s["at12"] < 1e4, s                          # the ppm window is the only way: the two isobars are never in one window
        step("no «Alta risoluzione» switch any more: a high-resolution file is always read in ppm", no_switch)
        def reload():
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(1500); pg.reload(); pg.wait_for_timeout(7000)
            tr = pg.evaluate("E.panels.filter(p=>p.type==='xic')[0].traces[0]"); print(tr)
            assert tr["ion"] and abs(tr["mz"] - 305.0702) < 1e-6, tr
        step("an ion trace comes back after a reload", reload)
        def lr_session():
            pg.evaluate("fetch('api/new',{method:'POST',body:JSON.stringify({fresh:true})})"); pg.reload(); pg.wait_for_timeout(1500)
            load(pg, [D / "B_FullMass-t0.mzML"], 4000, 1)
            a = pg.evaluate("(t=>HR.xicArgs(E.files[0],t,{tab:E.tab,tol:0.5}))(%s)" % __import__("json").dumps(T0)); print(a)
            assert abs(a[0] - 305.3) < 1e-6 and abs(a[1] - 0.5) < 1e-6, a   # QqQ: the unit window [304.8, 305.8] of today
        step("a QqQ file (its own session) keeps the unit window", lr_session)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
