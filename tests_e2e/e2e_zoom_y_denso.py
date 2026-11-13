"""Y zoom on the noisy baseline of dense traces (4 synthetic SRM files of 60 000 points, zoom 0-300 while the peaks reach 10^5): the trace strokes stay within a few
pixels of the plot (their peaks used to end thousands of plot heights above the canvas, 0.3 s per draw for 20 traces) and the picture is the one of the unclipped lines."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import bench_charts as BG  # noqa: E402  (its synthetic SRM file: noisy baseline, one peak per transition, one-point spikes)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_zoom_y_denso.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
WD = Path("/tmp/wd_zoom_y_denso"); WD.mkdir(parents=True, exist_ok=True)
FILES = []
for i in range(4):
    f = WD / f"G_MRM-y{i}.mzML"
    if not f.exists(): BG.srm_file(f, 60000, 2, 60 + i)
    FILES.append(f)
r = Run(port=8911, wd=str(WD / "run"))
P = "E.panels.find(q => q._a && q.type !== 'spec')"
# one draw with the y zoom: the lowest and highest y of every trace stroke (more than three vertices; the axes, baselines and markers have fewer), and the canvas height
SPAN = f"""(async () => {{ const p = {P}, C = CanvasRenderingContext2D.prototype, o = {{ b: C.beginPath, m: C.moveTo, l: C.lineTo, s: C.stroke }};
  let cur = []; const out = [];
  C.beginPath = function () {{ cur = []; return o.b.apply(this, arguments); }};
  C.moveTo = function (x, y) {{ cur.push(y); return o.m.apply(this, arguments); }}; C.lineTo = function (x, y) {{ cur.push(y); return o.l.apply(this, arguments); }};
  C.stroke = function () {{ if (cur.length > 3) out.push([Math.min(...cur), Math.max(...cur)]); return o.s.apply(this, arguments); }};
  try {{ await draw(p); }} finally {{ Object.assign(C, {{ beginPath: o.b, moveTo: o.m, lineTo: o.l, stroke: o.s }}); }}
  return {{ H: p.cv.clientHeight, out, n: p._a.sr.length }}; }})()"""
# the same draw with clipY switched off (the lines as they were): pixels that differ by more than 64 levels, and the largest shift of the top of the ink per column.
# The two can differ by anti-aliasing only: a software canvas rounds a stroke that ends 10^5 px away a little differently near the top edge of the plot.
SAME = f"""(async () => {{ const p = {P}, real = window.DECIMA ||= await import('./decima.js'), w = p.cv.width, h = p.cv.height;
  // read through one scratch canvas made for readback: repeated getImageData on the chart's own canvas makes Chromium log a willReadFrequently warning, a browser error for lib.Run
  const g = Object.assign(document.createElement('canvas'), {{ width: w, height: h }}).getContext('2d', {{ willReadFrequently: true }});
  const px = () => {{ g.clearRect(0, 0, w, h); g.drawImage(p.cv, 0, 0); return g.getImageData(0, 0, w, h).data; }};
  await draw(p); const a = px();
  window.DECIMA = {{ ...real, clipY: (/** @type {{number[]}} */ v) => v }}; try {{ await draw(p); }} finally {{ window.DECIMA = real; }}
  const b = px(); await draw(p);
  const ink = (d, x) => {{ for (let y = 0; y < h; y++) {{ const i = (y * w + x) * 4; if (Math.abs(d[i] - d[x * 4]) + Math.abs(d[i + 1] - d[x * 4 + 1]) + Math.abs(d[i + 2] - d[x * 4 + 2]) > 120) return y; }} return h; }};
  let n = 0, top = 0; for (let i = 0; i < a.length; i += 4) if (Math.max(Math.abs(a[i] - b[i]), Math.abs(a[i + 1] - b[i + 1]), Math.abs(a[i + 2] - b[i + 2])) > 64) n++;
  for (let x = 0; x < w; x++) top = Math.max(top, Math.abs(ink(a, x) - ink(b, x)));
  return {{ frac: n / (w * h), top }}; }})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(f) for f in FILES]); wait_for_js(pg, "E.files.length >= 4", 300000); ready(pg)
        pg.evaluate("E.files.forEach(f => f.vis = true); syncBrowse(); renderFileList(); redrawAll(); 0"); ready(pg)
        pg.evaluate(f"(() => {{ const p = {P}; p.zoomY = [0, 300]; return draw(p); }})()"); ready(pg)
        d = pg.evaluate(SPAN)
        def near():
            assert d["n"] == 4 and len(d["out"]) == 4, d
            far = [s for s in d["out"] if s[0] < -20 or s[1] > d["H"] + 20]
            assert not far, f"trace strokes reaching far beyond the {d['H']} px canvas: {far}"
        step("y zoom 0-300 on peaks of 10^5: every trace stroke stays within 20 px of the canvas", near)
        def same():
            q = pg.evaluate(SAME)
            assert q["frac"] < 0.005 and q["top"] <= 2, f"the clipped lines change the picture beyond anti-aliasing: {q}"
        step("the clipped lines draw the picture of the unclipped ones (anti-aliasing apart)", same)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
