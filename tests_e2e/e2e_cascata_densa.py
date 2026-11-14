"""Waterfall view of dense traces (4 synthetic SRM files of 60 000 points): each trace is drawn with about one vertex per pixel column, not one per point
(a waterfall of 20 files of 10^5 points took ~2.8 s per draw), and the apex of every trace is still drawn."""
# covers: mzlab/web/decima.js, mzlab/web/grafici.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import bench_charts as BG  # noqa: E402  (its synthetic SRM file: noisy baseline, one peak per transition, one-point spikes)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_cascata_densa.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
WD = Path("/tmp/wd_cascata_densa"); WD.mkdir(parents=True, exist_ok=True)
FILES = []
for i in range(4):
    f = WD / f"G_MRM-c{i}.mzML"
    if not f.exists(): BG.srm_file(f, 60000, 2, 40 + i)
    FILES.append(f)
r = Run(port=8907, wd=str(WD / "run"))
P = "E.panels.find(q => q._a && q.type !== 'spec')"
# one draw: every trace stroke (more than three vertices; the grey baselines and the small markers have fewer) with its vertex count and highest point (smallest y), and the
# first y of every filled path (the waterfall fills each trace from its own moved baseline Y(0) + sy, back to front, right before stroking it)
STROKES = f"""(async () => {{ const p = {P}, C = CanvasRenderingContext2D.prototype, o = {{ b: C.beginPath, m: C.moveTo, l: C.lineTo, s: C.stroke, f: C.fill }};
  let cur = []; const out = [], fills = [];
  C.beginPath = function () {{ cur = []; return o.b.apply(this, arguments); }};
  C.moveTo = function (x, y) {{ cur.push(y); return o.m.apply(this, arguments); }}; C.lineTo = function (x, y) {{ cur.push(y); return o.l.apply(this, arguments); }};
  C.stroke = function () {{ if (cur.length > 3) out.push([cur.length, Math.min(...cur)]); return o.s.apply(this, arguments); }};
  C.fill = function () {{ if (cur.length > 3) fills.push(cur[0]); return o.f.apply(this, arguments); }};
  try {{ await draw(p); }} finally {{ Object.assign(C, {{ beginPath: o.b, moveTo: o.m, lineTo: o.l, stroke: o.s, fill: o.f }}); }}
  const a = p._a, cs = a.sr.slice().sort((x, y) => ((E.files[x.k] || {{}}).time ?? 1e9) - ((E.files[y.k] || {{}}).time ?? 1e9) || x.k - y.k);
  const want = cs.slice().reverse().map((s, j) => {{ let m = -Infinity; for (const v of s.ys) if (v > m) m = v; return a.Y(a.U(s, m)) + (fills[j] - a.Y(0)); }});
  return {{ W: p.cv.clientWidth, out, want, cas: !!a.cas }}; }})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(f) for f in FILES]); wait_for_js(pg, "E.files.length >= 4", 300000); ready(pg)
        pg.evaluate("E.files.forEach(f => f.vis = true); syncBrowse(); renderFileList(); redrawAll(); 0"); ready(pg)
        pg.evaluate(f"(() => {{ const p = {P}; p.mode = 'cas'; return draw(p); }})()"); ready(pg)
        d = pg.evaluate(STROKES)
        def few():
            assert d["cas"] and len(d["want"]) == 4 and len(d["out"]) == 4, d
            big = [n for n, _ in d["out"] if n > 2 * d["W"] + 8]
            assert not big, f"strokes with more vertices than 2 per pixel column ({d['W']} px): {big}"
        step("waterfall of dense traces: at most two vertices per pixel column per trace", few)
        def apex():
            got = [y for _, y in d["out"]]
            assert len(got) == len(d["want"]) and all(abs(g - w) < 0.51 for g, w in zip(got, d["want"])), f"stroke tops {got} vs trace apexes {d['want']}"
        step("every trace keeps its apex (its tallest spike is the top of its line)", apex)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
