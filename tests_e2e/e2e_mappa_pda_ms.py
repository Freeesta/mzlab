"""DAD map, MS mode of the strips: the MS1 spectrum at the cursor RT plus the signed DAD->MS offset and the TIC of the same file; the offset is shown with its source, the
estimate shows its evidence and is applied only on request; a subtracted map takes the MS of the first file (or the reference when asked). Synthetic pairs, no laboratory data."""
# covers: mzlab/web/mappa.js, mzlab/web/mappa-dad-ms.js, mzlab/web/pannelli.js
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_pda_ms.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="mappa_pda_ms_"))
def pair(name, base, dpk, delay):
    w = wr.Writer(); n = 100; ts = [0.05 * k for k in range(n)]; wls = [200.0 + j for j in range(201)]
    tic = lambda t: 100 + sum(1000 * math.exp(-((t - (c + delay)) / 0.12) ** 2) for c in dpk)
    blocks = [("scan", [(base + k, int(tic(ts[k])))], 3000.0 * k, tic(ts[k])) for k in range(n)]
    rows = [[10 + sum(500 * math.exp(-((nm - 260) / 12) ** 2) * math.exp(-((t - c) / 0.12) ** 2) for c in dpk) for nm in wls] for t in ts]
    w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, 200.0, 1.0, rows)); w.save(D, name)
pair("a", 100, [1.0, 2.0, 3.0], 0.3)
pair("b", 500, [1.0, 2.0, 3.0], 0.3)
r = Run(port=8870, wd="/tmp/wd_mappa_pda_ms")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / f"{n}.wiff{x}") for n in "ab" for x in ("", ".scan")])
        pg.click("text=Load data"); ready(pg)
        pg.click("#np-map"); ready(pg)
        wait_for_js(pg, "E.files.length === 2 && E.files.every(f => f.pda)")
        pg.evaluate(f"(() => {{ const p = {M}; p.mapKind = 'pda'; p.k = 0; ctl(p); draw(p); }})()")
        wait_for_js(pg, f"{M}._a && {M}._a.pda && {M}._a.mat && window.MAPDAD")
        pg.evaluate(M + ".cv.focus()"); pg.keyboard.press("f"); wait_for_js(pg, M + ".el.classList.contains('max')"); ready(pg)
        def hover(rt, nm=260):
            xy = pg.evaluate(f"(() => {{ const a = {M}._a, c = {M}.cv.getBoundingClientRect(); return [c.left + a.X({rt}), c.top + a.Y({nm})]; }})()")
            pg.mouse.move(xy[0] - 5, xy[1] - 5); pg.mouse.move(*xy); ready(pg)
            wait_for_js(pg, f"(s => !!s && !!s.ms && Math.abs(s.pt.rt - {rt}) < 0.1)({M}._mz.shown)")
            wait_for_js(pg, f"{M}._mz.msSp && {M}._mz.msCh")
            return pg.evaluate(f"(s => ({{rt: s.ms.rt, off: s.ms.off, k: s.ms.k, mz: {M}._mz.msSp.mz, y: {M}._mz.msSp.y}}))({M}._mz.shown)")
        em = lambda base, k: wr.expected_scan([(base + k, 1)])[0][0]
        def scan_of(u, base=100):                       # the scan index the strip shows, from the m/z of its only peak
            mz = u["mz"][max(range(len(u["y"])), key=lambda i: u["y"][i])]; return min(range(100), key=lambda k: abs(em(base, k) - mz))
        pg.evaluate(f"(() => {{ const p = {M}; p.stripMs = 'ms'; ctl(p); draw(p); }})()")
        def zero():
            u = hover(1.0); assert u["off"] == 0 and abs(u["rt"] - 1.0) < 0.1, u; assert scan_of(u) == 20, scan_of(u)
            t = pg.evaluate(f"{M}.el.querySelector('.cpop').textContent"); assert "default" in t, t
        step("offset 0 by default (stated): the MS scan is the one at the DAD time", zero)
        def typed():
            pg.evaluate(f"(() => {{ const p = {M}; p.msOffset = 0.2; ctl(p); draw(p); }})()")
            u = hover(1.0); assert abs(u["off"] - 0.2) < 1e-9 and abs(u["rt"] - 1.2) < 0.1, u; assert scan_of(u) == 24, scan_of(u)
            assert "typed" in pg.evaluate(f"{M}.el.querySelector('.cpop').textContent")
        step("a typed signed offset moves the MS RT (+0.2 min -> scan 24)", typed)
        def est():
            pg.evaluate(f"{M}.el.querySelector('[data-o=msEst]').click()")
            wait_for_js(pg, f"{M}._msEst && !{M}._msEst.busy")
            d = pg.evaluate(f"{M}._msEst.d"); assert d and abs(d["delay"] - 0.3) <= 0.1 and d["peaks"] >= 2, d
            assert pg.evaluate(f"{M}.msOffset") == 0.2, "estimating alone changes nothing"
            t = pg.evaluate(f"{M}.el.querySelector('.cpop').textContent"); assert "correlation" in t and "peaks" in t, t
        step("the estimate shows its evidence and is not applied", est)
        def use():
            pg.evaluate(f"{M}.el.querySelector('[data-o=msUse]').click()"); ready(pg)
            assert abs(pg.evaluate(f"{M}.msOffset") - pg.evaluate(f"{M}._msEst.d.delay")) < 0.01
            assert "estimate" in pg.evaluate(f"{M}.el.querySelector('.cpop').textContent") and pg.evaluate(f"{M}._msLog.length") == 1
        step("using the estimate sets the offset and logs the choice", use)
        def subd():
            pg.evaluate(f"MAPPA.setCorr({M}, 'f1')"); wait_for_js(pg, f"{M}._a && {M}._a.ref && {M}._a.mat")
            u = hover(1.0); assert u["k"] == 0 and scan_of(u) in range(20, 27), u
            pg.evaluate(f"(() => {{ const p = {M}; p.msFrom = 'b'; draw(p); }})()")
            u = hover(1.0); assert u["k"] == 1 and scan_of(u, 500) in range(20, 27), u
        step("subtracted map: the MS is the first file's, the reference's when asked", subd)
        def traces():
            u = hover(1.0); mz = u["mz"][max(range(len(u["y"])), key=lambda i: u["y"][i])]
            pg.evaluate(f"(() => {{ const p = {M}; p.msTrace = 'bpc'; ctl(p); draw(p); }})()")
            wait_for_js(pg, f"{M}._mz.msCh && {M}._mz.msCh.key === {M}._mz.shown.ms.k + 'bpc'")
            pg.evaluate(f"(() => {{ const p = {M}; p.msTrace = 'xic'; p.msMz = ''; ctl(p); draw(p); }})()")
            wait_for_js(pg, f"{M}._mz.errCh === 'mappa.ms.noMz'")                 # no m/z typed: the reason is shown, not an endless wait
            pg.evaluate(f"(() => {{ const p = {M}; p.msMz = {mz}; ctl(p); draw(p); }})()")
            wait_for_js(pg, f"{M}._mz.errCh === null && {M}._mz.msCh && {M}._mz.msCh.key === {M}._mz.shown.ms.k + 'xic'")
            assert pg.evaluate(f"{M}._msLog.some(l => l.includes('MS level 1'))"), "the EIC window is logged"
            pg.evaluate(f"(() => {{ const p = {M}; p.msTrace = 'tic'; ctl(p); draw(p); }})()")
        step("BPC and EIC strips; an EIC without m/z says why; the EIC window is logged", traces)
        def back():
            pg.evaluate(f"(() => {{ const p = {M}; p.stripMs = 'uv'; ctl(p); draw(p); }})()")
            pg.mouse.move(300, 300); pg.mouse.move(310, 310); ready(pg)
            wait_for_js(pg, f"(s => !!s && !!s.uv)({M}._mz.shown)")
        step("back to UV: the UV strips return", back)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
