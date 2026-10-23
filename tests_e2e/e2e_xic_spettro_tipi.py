"""V-1 follow-up: the spectrum «XIC» button on a spectrum of the second file (as a map click opens it), with the files overlaid, and on an MS2 file (low resolution)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_xic_spettro_tipi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8877, wd="/tmp/wd77")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15")])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        ks = pg.evaluate("tabFiles('full').map(f=>f.k)"); assert len(ks) == 2, ks
        pg.evaluate(f"(()=>{{E.browse=false;tabFiles('full').forEach(f=>f.vis=true);renderFileList();redrawAll()}})()")
        def spec_of(k): return f"E.panels.find(p=>p.type==='spec'&&p.tab==='full'&&p.k=={k})"
        def top_peak(sp):      # the label of the highest labelled peak (away from the axes): a click on a label picks it whatever the height of the click
            pg.evaluate(f"{sp}.el.scrollIntoView({{block:'center'}})")      # the click is a real mouse click: the panel must be on screen
            frames(pg, 20)
            return pg.evaluate(f"(()=>{{const p={sp},q=p.cv.getBoundingClientRect(),l=p._a.lbls.filter(l=>l.m!=null).sort((u,v)=>u.y-v.y)[0];return {{mz:l.m,x:q.left+l.x+l.w/2,y:q.top+l.y+l.h/2}}}})()")
        def extract(sp):
            attendi(pg, f"!!{sp}&&!!{sp}._a&&{sp}._a.data[0].d.mz.length>0")
            pk = top_peak(sp)
            pg.evaluate(f"{sp}.el.querySelector('[data-a=xicb]').click()"); pg.mouse.click(pk["x"], pk["y"]); ready(pg)
            attendi(pg, "(()=>{const x=E.panels.find(p=>p.type==='xic');return !!x&&!!x._a&&!!x._a.sr&&x._a.sr.length>0})()")
            return pk
        def second_file():
            pg.evaluate(f"newSpec(E.panels.find(p=>p.type==='chrom'&&p.tab==='full'), 14.9, 15.1, {ks[1]})"); ready(pg)
            attendi(pg, f"!!{spec_of(ks[1])}")
            pk = extract(spec_of(ks[1]))
            sr = pg.evaluate("E.panels.find(p=>p.type==='xic')._a.sr.map(s=>s.k)")
            assert ks[1] in sr, (sr, ks)
            assert pg.evaluate("E.panels.filter(p=>p.type==='xic').length") == 1
        step("spectrum of the second file (as a map click opens it): the XIC is of that file", second_file)
        def overlaid():
            sp = spec_of(ks[0]); pg.evaluate(f"{sp}.el.querySelector('input[data-o=all]').click()"); ready(pg)
            extract(sp)
            sr = sorted(pg.evaluate("E.panels.find(p=>p.type==='xic')._a.sr.map(s=>s.k)"))
            print("series", sr)
            assert ks[0] in sr, (sr, ks)
        step("files overlaid: the XIC of the chosen peak is drawn", overlaid)
        def ms2():
            open_kind(pg, "ms2")
            attendi(pg, "(()=>{const p=E.panels.find(q=>q.tab==='ms2'&&q.type==='spec');return !!p&&!!p._a&&p._a.data[0].d.mz.length>0})()")
            sp = "E.panels.find(p=>p.type==='spec'&&p.tab==='ms2')"
            n0 = pg.evaluate("E.panels.filter(p=>p.type==='xic'&&p.tab==='ms2').length")
            pg.evaluate(f"{sp}.el.scrollIntoView({{block:'center'}})"); frames(pg, 20)
            pg.evaluate(f"{sp}.el.querySelector('[data-a=xicb]').click()")
            pk = top_peak(sp)
            # the start screen fades out over 0.4 s and swallows the clicks meanwhile: wait until the canvas is what lies under the pointer
            attendi(pg, f"document.elementFromPoint({pk['x']},{pk['y']})==={sp}.cv")
            pg.mouse.click(pk["x"], pk["y"]); ready(pg)
            attendi(pg, "E.panels.some(p=>p.type==='xic'&&p.tab==='ms2'&&p._a&&p._a.sr&&p._a.sr.length)")
            t = pg.evaluate("E.panels.find(p=>p.type==='xic'&&p.tab==='ms2').traces[0].mz")
            assert abs(t - pk["mz"]) < 1.0, (t, pk["mz"])
        step("MS2 file (full scan filter): the XIC of the fragment is drawn in the MS2 tab", ms2)
        step("STATO.verifica() is empty", lambda: (lambda v: v and (_ for _ in ()).throw(AssertionError(v)))(pg.evaluate("typeof STATO!=='undefined'&&STATO.verifica?STATO.verifica():[]")))
        errs = [e for e in r.errs if "iframe" not in str(e)]
        for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
        print("--- BROWSER ERRORS", len(errs)); [print(e) for e in errs[:5]]
finally: r.close()
