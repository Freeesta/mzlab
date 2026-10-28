"""A spectrum with 200 000 centroids and a 200 000-point profile: drawn without RangeError (no Math.max(...a) on the arrays), the nearest peak under the pointer
equals the brute-force one, the intensity axis fits the window, and the same holds when the m/z array is not ascending."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_spettro_grande.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
# api/spectrum answers with the real keys of the engine, the arrays replaced by synthetic ones (window.__SYN.shuffle = 1: the centroids are not in m/z order)
SYN = """
(() => {
  const N = 200000; let seed = 7; const rnd = () => (seed = (seed * 1664525 + 1013904223) >>> 0) / 4294967296;
  const pmz = new Array(N), py = new Array(N), mz = new Array(N), y = new Array(N);
  for (let i = 0; i < N; i++) { pmz[i] = 100 + i * 900 / N; py[i] = 5 * rnd() + (i % 400 < 8 ? 3e4 : 0); mz[i] = 100.0004 + i * 899.9 / N + rnd() * 0.001; y[i] = 10 + rnd() * 1e5; }
  window.__SYN = { pmz, py, mz, y, shuffle: 0 };
  const of = window.fetch;
  window.fetch = function (u, o) {
    const url = typeof u === "string" ? u : u.url, s = window.__SYN;
    if (/api\\/spectrum\\?/.test(url)) return of.call(this, u, o).then(r => r.json()).then(j => {
      let m = s.mz, v = s.y;
      if (s.shuffle) { const idx = m.map((_, i) => i).sort((a, b) => ((a * 7919) % 1009) - ((b * 7919) % 1009) || a - b); m = idx.map(i => s.mz[i]); v = idx.map(i => s.y[i]); }
      return new Response(JSON.stringify({ ...j, mz: m, y: v, pmz: s.pmz, py: s.py }), { headers: { "content-type": "application/json" } });
    });
    return of.call(this, u, o);
  };
})()
"""
CHECK = """
async (z) => {
  const sp = E.panels.filter(q => q.type === 'spec').pop(); sp.zoom = z; sp.zoomY = null; await drawSpec(sp);
  const a = sp._a, d = a.data[0].d, x0 = a.x0, x1 = a.x1, W = a.W;
  // the old rules, brute force: top of the axis, nearest bar (low resolution branch of snap)
  let top = 1e-9; for (let i = 0; i < d.pmz.length; i++) if (d.pmz[i] >= x0 && d.pmz[i] <= x1 && d.py[i] > top) top = d.py[i];
  const lineOf = !!d.pmz && !sp.sticks;
  if (!lineOf) { top = 1e-9; for (let i = 0; i < d.mz.length; i++) if (d.mz[i] >= x0 && d.mz[i] <= x1 && d.y[i] > top) top = d.y[i]; }
  const old = px => {
    let best = null;
    for (let j = 0; j < d.mz.length; j++) { const m = d.mz[j]; if (m < x0 || m > x1 || d.y[j] <= 0) continue; const dd = Math.abs(a.X(m) - px); if (dd < 14 && (!best || dd < best.dd)) best = { dd, m, j }; }
    if (!best) return null;
    let jb = best.j; for (let j = 0; j < d.mz.length; j++) if (Math.abs(d.mz[j] - best.m) <= 0.35 && d.y[j] > d.y[jb]) jb = j;
    let sw = 0, sm = 0; for (let j = 0; j < d.mz.length; j++) if (Math.abs(d.mz[j] - d.mz[jb]) <= 0.5 && d.y[j] > 0) { sw += d.y[j]; sm += d.mz[j] * d.y[j]; }
    return [sw > 0 ? sm / sw : d.mz[jb], d.y[jb]];
  };
  let bad = 0, n = 0, hit = 0;
  for (let px = 70; px < W - 20; px += 37) { const s = a.snap(px), o = old(px); n++; if (o) hit++; if (!!s !== !!o || (s && (s.m !== o[0] || s.y !== o[1]))) bad++; }
  return { n, hit, bad, ymax: a.yfull[1], want: top * 1.12 / (1 - (sp._legPad || 0)), pts: d.mz.length, ink: sp.cv.toDataURL().length > 3000 };
}
"""
r = Run(port=8894, wd="/tmp/wd94")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz("B_FullMass-t0")]); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate(SYN)
        pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1,r0:5.0,r1:5.4,h:420})"); ready(pg)
        def sorted_case():
            for z in (None, [300, 600], [400.0, 420.0], [700, 700.8]):
                c = pg.evaluate(CHECK, z)
                assert c["pts"] == 200000 and c["ink"], c
                assert c["bad"] == 0, f"snap differs from the brute force at zoom {z}: {c}"
                assert abs(c["ymax"] - c["want"]) <= 1e-9 * max(1, c["want"]), f"axis top at zoom {z}: {c}"
            assert c["hit"] >= 1, "no hover case was exercised"
        step("200 000 centroids and points: drawn, nearest peak and axis as the brute force", sorted_case)
        def shuffled_case():
            pg.evaluate("window.__SYN.shuffle = 1; E.panels.filter(q => q.type === 'spec').forEach(closePanel); addPanel('spec',{tab:E.tab,k:E.cur,level:1,r0:5.0,r1:5.4,h:420})"); ready(pg)
            for z in (None, [400.0, 420.0]):
                c = pg.evaluate(CHECK, z)
                assert c["bad"] == 0 and c["pts"] == 200000 and c["ink"], c
                assert abs(c["ymax"] - c["want"]) <= 1e-9 * max(1, c["want"]), c
        step("m/z not in order: the same answers (no binary search is trusted)", shuffled_case)
        def clean():
            bad = [e for e in r.errs if e[0] in ("pageerror", "console.error") and "GL Driver" not in str(e)]
            assert not bad, bad[:3]
        step("no page error", clean)
finally:
    r.close(); r.report()
bad = [s for s in steps if s[1] != "ok"]
for n, st in steps: print(("FAIL " if st != "ok" else "ok   ") + n + ("" if st == "ok" else " -> " + st))
if bad: sys.exit(1)
