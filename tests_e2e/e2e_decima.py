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
# one panel: the picture with decima.js, then with the point-by-point scans (the old loop), at the full view and at three zooms
SHOTS = """async ([id, fr]) => {
  const p = E.panels.find(q => q.id === id), out = [];
  const real = window.DECIMA ||= await import('./decima.js'), old = { ...real, polyline: real.polylineScan, extent: real.extentScan };
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
        for label, f, nf in (("LR", "B_FullMass-t0.mzML", 1), ("HR", "HR_DDA-Exploris-t30.mzML", 2)):
            pg = r.page(p)
            load(pg, [D / f], 6000, nf); ready(pg)
            def tic():
                ids = pg.evaluate("E.panels.filter(p => p.type === 'chrom' && p.cv.clientWidth > 30).map(p => p.id)"); assert ids, "no chromatogram panel"
                for i in ids:
                    ok = pg.evaluate(SHOTS, [i, None]); assert all(ok), f"panel {i}: pictures differ at zoom {ok}"
            step(f"{label} TIC: same picture as the old loop at 4 zooms", tic)
            def xic():
                pg.evaluate("xicDirect(301.1, null)"); ready(pg); frames(pg, 4)
                ids = pg.evaluate("E.panels.filter(p => p.type === 'xic' && p._a).map(p => p.id)"); assert ids, "no XIC panel"
                for i in ids:
                    ok = pg.evaluate(SHOTS, [i, None]); assert all(ok), f"xic {i}: pictures differ at zoom {ok}"
            step(f"{label} XIC: same picture as the old loop at 4 zooms", xic)
            pg.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
