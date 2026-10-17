"""Sight of the map with a file subtracted (two synthetic example files, B subtracted from A): at an RT the side plot holds the whole difference spectrum A − B
(every m/z, positive and negative; the sum of the differences is the difference of the sums), at an m/z the bottom plot holds the XIC A − B in time, and the TIC
A − B is read from the two TIC; the colours are the variables of the difference of the map (--diff-pos, --diff-neg)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_mirino_diff.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8902, wd="/tmp/wd_mirino_diff")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1280, "height": 950})
        ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1')}catch(_){}")
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/")
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")]); ready(pg)
        pg.click("#opbtn"); ready(pg)
        pg.click("#np-map"); ready(pg)
        pg.evaluate(f"(()=>{{const q={M};E.cur=E.files[1].k;q.k=E.files[1].k;q.corr='file';q.ref=E.files[0].k;ctl(q);draw(q)}})()"); ready(pg)
        attendi(pg, f"!!({M} && {M}._a && {M}._a.ref && {M}.cv)")
        pg.evaluate(M + ".el.scrollIntoView({block:'start'})")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("m")
        c = pg.evaluate(f"(()=>{{const c={M}.cv.getBoundingClientRect();return [c.left,c.top,c.width,c.height]}})()")
        pg.mouse.move(c[0] + c[2] * .5, c[1] + c[3] * .5); pg.mouse.move(c[0] + c[2] * .52, c[1] + c[3] * .5)
        attendi(pg, f"(()=>{{const s={M}._mz; return !!(s.shown && s.shown.sp && s.shown.xi && s.tic)}})()")
        def spectrum():
            sp = pg.evaluate(f"({{...{M}._mz.spec, mz: undefined, y: undefined, n: {M}._mz.spec.mz.length, sum: {M}._mz.spec.y.reduce((a, b) => a + b, 0), pos: {M}._mz.spec.y.filter(v => v > 0).length, neg: {M}._mz.spec.y.filter(v => v < 0).length}})")
            assert sp["signed"] and sp["n"] > 3, sp
            sa = pg.evaluate(f"(async()=>{{const m={M}, a=m._a, ax=await (await fetch('api/scan_axis?k='+a.f.k+'&level='+(a.f.lv||1))).json(), bx=await (await fetch('api/scan_axis?k='+a.ref.k+'&level='+(a.f.lv||1))).json(); const near=(r,t)=>r.reduce((b,v,i)=>Math.abs(v-t)<Math.abs(r[b]-t)?i:b,0); const t=m._mz.spec.rt; const get=async(k,i)=>(await (await fetch('api/spectra?k='+k+'&i0='+i+'&i1='+i+'&level='+(a.f.lv||1)+'&bin=0.1')).json()).scans[0].y.reduce((s,v)=>s+v,0); return await get(a.f.k, near(ax.rt,t)) - await get(a.ref.k, near(bx.rt,t))}})()")
            assert abs(sp["sum"] - sa) <= 1e-6 * max(1, abs(sa)), (sp["sum"], sa)              # the sum of A − B is the sum of A minus the sum of B
        step("the spectrum of the sight is the difference A − B of the two scans", spectrum)
        def xic():
            d = pg.evaluate(f"(()=>{{const s={M}._mz; return {{tr: s.xic.tr.length, tn: s.tic.rt.length, drawn: !!s.xc}}}})()")
            assert d["tr"] == 2 and d["tn"] > 10, d                                          # both XIC are asked, the difference is drawn from them; the TIC A − B exists
            assert pg.evaluate("getComputedStyle(document.documentElement).getPropertyValue('--diff-pos').trim()") != ""
        step("the XIC and the TIC differences are there", xic)
        def plots():
            assert pg.evaluate(f"{M}.rd.textContent.includes(' − ')"), pg.evaluate(f"{M}.rd.textContent")   # the readout says A − B
        step("the readout names A − B", plots)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
