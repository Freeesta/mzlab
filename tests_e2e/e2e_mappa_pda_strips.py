"""Strips of the DAD map: the right strip is the UV spectrum at the cursor time and the bottom strip the UV trace at the cursor wavelength, read from the matrix
(plain and subtracted), never from the MS routes. Synthetic pairs (tools/wiff_riferimento.py), no laboratory data."""
# covers: mzlab/web/mappa.js, mzlab/web/grafici.js
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_pda_strips.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="mappa_pda_strips_"))
def pair(name, n, dt, wl0, wl1, step_nm, peaks):
    w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(n)]
    ts = [dt * k for k in range(n)]; wls = [wl0 + step_nm * j for j in range(int((wl1 - wl0) / step_nm) + 1)]
    rows = [[10 + sum(h * math.exp(-((nm - c) / 12) ** 2) * math.exp(-((t - rt) / 0.2) ** 2) for h, c, rt in peaks) for nm in wls] for t in ts]
    w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, wl0, step_nm, rows)); w.save(D, name)
pair("a", 40, 0.05, 200.0, 400.0, 1.0, [(400, 260, 1.0)])
pair("b", 40, 0.05, 200.0, 400.0, 1.0, [(300, 300, 1.5)])
r = Run(port=8869, wd="/tmp/wd_mappa_pda_strips")
M = "E.panels.find(q=>q.type==='map')"
COUNT = "(()=>{window.__rq=[];const f=window.fetch;window.fetch=function(u,...a){const m=/api\\/([a-z_]+)/.exec(String(u));if(m)window.__rq.push(m[1]);return f.call(this,u,...a)}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / f"{n}.wiff{x}") for n in "ab" for x in ("", ".scan")])
        pg.click("text=Load data"); ready(pg)
        pg.click("#np-map"); ready(pg)
        wait_for_js(pg, "E.files.length === 2 && E.files.every(f => f.pda)")
        pg.evaluate(f"(() => {{ const p = {M}; p.mapKind = 'pda'; p.k = 0; ctl(p); draw(p); }})()")
        wait_for_js(pg, f"{M}._a && {M}._a.pda && {M}._a.mat")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); wait_for_js(pg, M + ".el.classList.contains('max')"); ready(pg)
        pg.evaluate(COUNT)
        def hover(rt, nm):
            xy = pg.evaluate(f"(() => {{ const a = {M}._a, c = {M}.cv.getBoundingClientRect(); return [c.left + a.X({rt}), c.top + a.Y({nm})]; }})()")
            pg.mouse.move(xy[0] - 5, xy[1] - 5); pg.mouse.move(*xy); ready(pg)
            wait_for_js(pg, f"(s => !!s && !!s.uv && Math.abs(s.pt.rt - {rt}) < 0.1)({M}._mz.shown)")
            return pg.evaluate(f"(s => ({{sp: s.uv.spec, xic: s.uv.xic}}))({M}._mz.shown)")
        def argmax(xs, ys): return xs[max(range(len(ys)), key=lambda i: ys[i])]
        def plain():
            u = hover(1.0, 260)
            assert u["sp"]["uv"] and u["xic"]["uv"], u
            assert abs(argmax(u["sp"]["mz"], u["sp"]["y"]) - 260) <= 2, "spectrum peak at 260 nm"
            assert abs(argmax(u["xic"]["tr"][0]["rt"], u["xic"]["tr"][0]["y"]) - 1.0) <= 0.06, "trace peak at 1.0 min"
            assert max(u["sp"]["y"]) > 300 and max(u["xic"]["tr"][0]["y"]) > 300
        step("plain map: the strips hold the UV spectrum at the cursor time and the UV trace at the cursor wavelength", plain)
        def noms():
            assert not [x for x in pg.evaluate("window.__rq") if x in ("xic", "spectra", "scan_axis")], pg.evaluate("window.__rq")
        step("no MS route is asked for the strips of a DAD map", noms)
        def sub():
            pg.evaluate(f"MAPPA.setCorr({M}, 'f1')"); wait_for_js(pg, f"{M}._a && {M}._a.ref && {M}._a.mat")
            u = hover(1.5, 300)
            assert u["sp"]["signed"] and u["xic"]["signed"], u
            assert min(u["sp"]["y"]) < -200 and min(u["xic"]["tr"][0]["y"]) < -200, "B's peak is negative in both strips"
            assert abs(u["sp"]["mz"][min(range(len(u["sp"]["y"])), key=lambda i: u["sp"]["y"][i])] - 300) <= 2
        step("subtracted map: both strips are signed and show B's peak below zero", sub)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
