"""Chromatograms and XICs drawn from the zoom window (decima.js) are pixel-identical to the point-by-point loop kept for unsorted time axes:
LR and HR files, full view and three partial zooms, TIC and XIC."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_decima.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("decima")
r = Run(port=8866, wd="/tmp/wd_decima")
# one panel: the picture with decima.js, then with isSorted() false (the old loop), at the full view and at three zooms
SHOTS = """async ([id, fr]) => {
  const p = E.panels.find(q => q.id === id), out = [];
  const real = window.DECIMA ||= await import('./decima.js'), old = { ...real, isSorted: () => false };
  const a0 = p._a; const lo = a0.full[0], hi = a0.full[1];
  for (const z of [null, [lo + (hi - lo) * 0.3, lo + (hi - lo) * 0.5], [lo + (hi - lo) * 0.05, lo + (hi - lo) * 0.12], [hi - (hi - lo) * 0.4, hi]]) {
    p.zoom = z; window.DECIMA = real; await draw(p); const n = p.cv.toDataURL();
    window.DECIMA = old; await draw(p); const o = p.cv.toDataURL();
    out.push(n === o);
  }
  window.DECIMA = real; p.zoom = null; await draw(p);
  return out;
}"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML", D / "HR_DDA-Exploris-t30.mzML"], 6000, 3); ready(pg)
        def panels():
            return pg.evaluate("E.panels.filter(p => p.type === 'chrom' && p.cv.clientWidth > 30).map(p => p.id)")
        def tic():
            ids = panels(); assert ids, "no chromatogram panel"
            for i in ids:
                ok = pg.evaluate(SHOTS, [i, None]); assert all(ok), f"panel {i}: pictures differ at zoom {ok}"
        step("TIC of LR and HR files: same picture as the old loop at 4 zooms", tic)
        def xic():
            pid = pg.evaluate("(() => { const q = xicDirect(E.files[0].mz0 || 301.1, null); return q ? q.id : null; })()")
            ready(pg); frames(pg, 4)
            ids = pg.evaluate("E.panels.filter(p => p.type === 'xic' && p._a).map(p => p.id)"); assert ids, "no XIC panel"
            for i in ids:
                ok = pg.evaluate(SHOTS, [i, None]); assert all(ok), f"xic {i}: pictures differ at zoom {ok}"
        step("XIC: same picture as the old loop at 4 zooms", xic)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
