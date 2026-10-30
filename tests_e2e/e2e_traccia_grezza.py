"""Raw (unsmoothed) chromatograms: a dense trace is a thin line of column peaks, never a solid band; smoothing keeps its drawing; a zoom shows the true points."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_traccia_grezza.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "traccia_grezza_tmp"; D.mkdir(parents=True, exist_ok=True)
F = D / "BK_t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(3))
# the pixel test draws a noisy 90 000-point trace with the helper of the panels and counts the columns whose ink fills most of the plot height
SOLID = """async (mode) => {
  const DC = window.DECIMA || await import('/static/decima.js'); const n = 90000, x = new Float64Array(n), y = new Float64Array(n); let s = 12345;
  const rnd = () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; };
  for (let i = 0; i < n; i++) { x[i] = i / n * 15; const g = Math.sqrt(-2 * Math.log(rnd() + 1e-12)) * Math.cos(6.283 * rnd()); y[i] = Math.abs(g) * 3e8 + (rnd() < .01 ? 2e9 : 0) + 1.5e9 * Math.exp(-((x[i] - 7) ** 2) / .1); }
  const W = 900, H = 240, X = t => 40 + t / 15 * (W - 60), V = v => H - 20 - v / 3.5e9 * (H - 40);
  const c = document.createElement('canvas'); c.width = W; c.height = H; const g = c.getContext('2d'); g.fillStyle = '#fff'; g.fillRect(0, 0, W, H);
  let v, lw = 1.25, t0 = performance.now(); if (mode === 'old') v = DC.polyline(x, y, 0, 15, X, V); else { const o = DC.rawLine(x, y, 0, 15, X, V); v = o.v; lw = o.thin ? 1 : 1.25; }
  const ms = performance.now() - t0;
  g.strokeStyle = '#0072B2'; g.lineWidth = lw; g.beginPath(); for (let k = 0; k < v.length; k += 2) k ? g.lineTo(v[k], v[k + 1]) : g.moveTo(v[0], v[1]); g.stroke();
  const d = g.getImageData(0, 0, W, H).data; let solid = 0, cols = 0, filled = 0;
  for (let cx = 40; cx < W - 20; cx++) { cols++; let ink = 0; for (let cy = 0; cy < H; cy++) if (d[(cy * W + cx) * 4] < 200) ink++; if (ink > .4 * (H - 40)) solid++; }
  for (let cx = 40; cx < W - 20; cx++) { let top = -1, ink = 0; for (let cy = 0; cy < H - 20; cy++) if (d[(cy * W + cx) * 4] < 200) { if (top < 0) top = cy; ink++; } if (top >= 0 && ink > .9 * (H - 20 - top)) filled++; }
  return { solid: solid / cols, filled: filled / cols, verts: v.length / 2, ms, lw }; }"""
r = Run(port=8981, wd="/tmp/wd_traccia_grezza")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)]); pg.wait_for_selector("text=Carica dati"); pg.click("text=Carica dati"); ready(pg)
        def band():
            old, new = pg.evaluate(SOLID, "old"), pg.evaluate(SOLID, "raw")
            assert old["solid"] > .6, ("the control (first/lowest/highest/last per column) must be the solid band", old)
            assert new["solid"] < .35 and new["filled"] < .35, ("the raw trace is still a solid mass", new)
            assert new["lw"] == 1 and new["verts"] <= 900, new
            assert new["ms"] < 50, ("the 90 000-point trace takes too long to reduce", new)
        step("a noisy 90 000-point raw trace: fewer than 35% of the columns are solid, 1 px, one vertex per column", band)
        P = "E.panels.find(q=>q.tab==='full'&&q.type==='chrom')"
        def routing():
            n = pg.evaluate(f"""(async () => {{ const DC = window.DECIMA, c = {{raw: 0, poly: 0}};       // a module namespace is read-only: the panels read window.DECIMA, so a wrapper replaces it
              window.DECIMA = {{...DC, rawLine: (...a) => (c.raw++, DC.rawLine(...a)), polyline: (...a) => (c.poly++, DC.polyline(...a))}}; const p = {P};
              p.smooth = false; await draw(p); const off = {{...c}}; c.raw = c.poly = 0; p.smooth = true; await draw(p); const on = {{...c}};
              window.DECIMA = DC; return {{off, on}}; }})()""")
            assert n["off"]["raw"] >= 1 and n["off"]["poly"] == 0 and n["on"]["poly"] >= 1 and n["on"]["raw"] == 0, n
        step("every panel reaches the one helper: raw traces use rawLine, smoothed ones keep polyline", routing)
        def zoom():
            v = pg.evaluate("""(() => { const DC = window.DECIMA, n = 400, x = Float64Array.from({length: n}, (_, i) => i), y = Float64Array.from({length: n}, (_, i) => i % 7);
              const a = DC.rawLine(x, y, 0, n - 1, t => t * 2, v => -v), b = DC.rawLine(x, y, 100, 120, t => (t - 100) * 40, v => -v); return [a.thin, b.thin, b.v.length / 2]; })()""")
            assert v[0] is False and v[1] is False and v[2] >= 21, v
        step("with up to 1.5 points per pixel (zoomed in) the true points are drawn", zoom)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
