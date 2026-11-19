"""3D view of the DAD map: it shows the DAD matrix (axes in nm, plain and subtracted), never the MS map of the file. Where the browser has no WebGL2 the viewer
must say so in its technical box (that path is asserted instead). Synthetic pairs (tools/wiff_riferimento.py), no laboratory data."""
# covers: mzlab/web/mappa3d.js, mzlab/web/mappa-dad.js
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_pda_3d.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="mappa_pda_3d_"))
def pair(name, n, dt, wl0, wl1, step_nm, peaks):
    w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(n)]
    ts = [dt * k for k in range(n)]; wls = [wl0 + step_nm * j for j in range(int((wl1 - wl0) / step_nm) + 1)]
    rows = [[10 + sum(h * math.exp(-((nm - c) / 12) ** 2) * math.exp(-((t - rt) / 0.2) ** 2) for h, c, rt in peaks) for nm in wls] for t in ts]
    w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, wl0, step_nm, rows)); w.save(D, name)
pair("a", 40, 0.05, 200.0, 400.0, 1.0, [(400, 260, 1.0)])
pair("b", 40, 0.05, 200.0, 400.0, 1.0, [(300, 300, 1.5)])
r = Run(port=8870, wd="/tmp/wd_mappa_pda_3d")
M = "E.panels.find(q=>q.type==='map')"
COUNT = "(()=>{window.__rq=[];const f=window.fetch;window.fetch=function(u,...a){if(/api\\/map/.test(String(u)))window.__rq.push(String(u));return f.call(this,u,...a)}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / f"{n}.wiff{x}") for n in "ab" for x in ("", ".scan")])
        pg.click("text=Load data"); ready(pg)
        pg.click("#np-map"); ready(pg)
        wait_for_js(pg, "E.files.length === 2 && E.files.every(f => f.pda)")
        pg.evaluate(f"(() => {{ const p = {M}; p.mapKind = 'pda'; p.k = 0; ctl(p); draw(p); }})()")
        wait_for_js(pg, f"{M}._a && {M}._a.pda && {M}._a.mat")
        pg.evaluate(COUNT)
        gl = pg.evaluate("!!document.createElement('canvas').getContext('webgl2')")
        def outcome(label):
            pg.evaluate(f"MAPPA.open3d({M}, document.body)")
            if gl:
                wait_for_js(pg, "!!document.querySelector('dialog.m3d .m3d-read') && /nm/.test(document.querySelector('dialog.m3d .m3d-read').textContent)")
                wait_for_js(pg, "document.querySelector('dialog.m3d [data-m=res]').textContent.length > 0")
            else:
                wait_for_js(pg, "!!document.querySelector('.m3d-tech')")
        def plain():
            outcome("plain")
            if gl:
                res = pg.evaluate("document.querySelector('dialog.m3d [data-m=res]').textContent"); assert "40" in res, res
                rd = pg.evaluate("document.querySelector('dialog.m3d .m3d-read').textContent"); assert "m/z" not in rd and "nm" in rd, rd
                assert not [u for u in pg.evaluate("window.__rq") if "level=" in u], pg.evaluate("window.__rq")      # the MS map is never asked
        step("3D of the plain DAD map: the matrix in nm, no MS map request" if gl else "no WebGL2 here: the viewer explains it in its technical box", plain)
        def close():
            pg.evaluate("document.querySelectorAll('dialog.m3d,.m3d-tech').forEach(x => (x.close ? x.close() : 0)); document.querySelectorAll('dialog.m3d').forEach(x => x.remove())")
        close()
        if gl:
            def sub():
                pg.evaluate(f"MAPPA.setCorr({M}, 'f1')"); wait_for_js(pg, f"{M}._a && {M}._a.ref && {M}._a.pda")
                pg.evaluate("window.__rq = []"); outcome("sub")
                wait_for_js(pg, "/ − /.test(document.querySelector('dialog.m3d .m3d-read').textContent)")
                assert not [u for u in pg.evaluate("window.__rq") if "level=" in u], pg.evaluate("window.__rq")
            step("3D of the subtracted DAD map: A − B in nm", sub)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
