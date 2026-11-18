"""DAD map minus the DAD map of another file in the browser: the reference is resampled onto the grid of the first file (different time steps and wavelength ranges),
the map shows the difference with the peak only the first file has, and a visible note tells the user what was aligned, dropped and scaled. Synthetic pairs
(tools/wiff_riferimento.py), no laboratory data."""
# covers: mzlab/web/grafici.js
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_pda_sub.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="mappa_pda_sub_"))
def pair(name, n, dt, wl0, wl1, step_nm, peaks):
    w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(n)]
    ts = [dt * k for k in range(n)]; wls = [wl0 + step_nm * j for j in range(int((wl1 - wl0) / step_nm) + 1)]
    rows = [[10 + sum(h * math.exp(-((nm - c) / 12) ** 2) * math.exp(-((t - rt) / 0.2) ** 2) for h, c, rt in peaks) for nm in wls] for t in ts]
    w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, wl0, step_nm, rows)); w.save(D, name)
pair("a", 40, 0.05, 200.0, 400.0, 1.0, [(400, 260, 1.0)])                 # A: a peak at 1.0 min, 260 nm
pair("b", 30, 0.07, 220.0, 380.0, 2.0, [(300, 260, 1.5)])                 # B: another grid (step, range), a peak at 1.5 min, 260 nm
r = Run(port=8868, wd="/tmp/wd_mappa_pda_sub")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / f"{n}.wiff{x}") for n in "ab" for x in ("", ".scan")])
        pg.click("text=Load data"); ready(pg)
        pg.click("#np-map"); ready(pg)
        def plain():
            wait_for_js(pg, "E.files.length === 2 && E.files.every(f => f.pda)")
            pg.evaluate(f"(() => {{ const p = {M}; p.mapKind = 'pda'; p.k = 0; ctl(p); draw(p); }})()")
            wait_for_js(pg, f"{M}._a && {M}._a.pda && {M}._a.mat && {M}._a.mat.nrt === 40")
        step("the plain DAD map of the first file is drawn", plain)
        def diff():
            pg.evaluate(f"(() => {{ const p = {M}; MAPPA.setCorr(p, 'f1'); }})()")
            wait_for_js(pg, f"{M}._a && {M}._a.pda && {M}._a.ref && {M}._a.ref.k === 1")
            up = pg.evaluate(f"{M}._a.mapv(1.0, 260)"); down = pg.evaluate(f"{M}._a.mapv(1.5, 260)")
            assert up is not None and up > 300, up                               # the peak only A has
            assert down is not None and down < -200, down                        # the peak only B has: negative
        step("choosing a file subtracts its DAD map: A's peak up, B's peak down (not a silent no-op)", diff)
        def note():
            t = pg.evaluate(f"[...{M}.el.querySelectorAll('.subnote')].map(e => e.title + ' ' + e.textContent).join(' ')")
            assert "on the grid of" in t and "wavelengths" in t, t[:400]
            assert "interpolated" in t, t[:600]                                  # B has other steps: the user is told
        step("a visible note says what was kept, interpolated and scaled", note)
        def scaled():
            pg.evaluate(f"(() => {{ const p = {M}; p.subFactor = 0.5; draw(p); }})()")
            wait_for_js(pg, f"[...{M}.el.querySelectorAll('.subnote')].some(e => e.textContent.includes('×0.5'))")
            d2 = pg.evaluate(f"{M}._a.mapv(1.5, 260)"); assert d2 > -200 and d2 < -50, d2       # half of B subtracted
        step("a typed scale factor changes the difference and is shown", scaled)
        def nooverlap():
            pg.evaluate(f"(() => {{ const p = {M}; p.subFactor = null; p.subShift = 50; draw(p); }})()")
            wait_for_js(pg, f"[...{M}.el.querySelectorAll('.subnote')].some(e => e.textContent.includes('Cannot subtract'))")
            assert pg.evaluate(f"!{M}._a.ref")                                   # the plain map stays, with the reason written
        step("a shift with no overlap says so and keeps the plain map", nooverlap)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
