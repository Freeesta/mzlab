"""Zooming a chromatogram in and out many times keeps memory flat: every header rebuild must let the old header controls go
(an abort() DOMException in corrInfoWire kept them, about 490 DOM nodes per zoom-unzoom cycle). A synthetic LR file; nodes and listeners after GC."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_memoria_zoom.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("memoria_zoom")
r = Run(port=8916, wd="/tmp/wd_memoria_zoom")
CYC = """async n => {
  const p = E.panels.find(q => q.type === 'chrom' && q._a), f = p._a.full;
  for (let i = 1; i <= n; i++) { p.zoom = [f[0] + (f[1] - f[0]) * 0.1 * (i % 5 + 1), f[1]]; await draw(p); p.zoom = null; await draw(p); }
}"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "B_FullMass-t0.mzML"], 6000, 1); ready(pg)
        cdp = pg.context.new_cdp_session(pg); cdp.send("Performance.enable")
        def metrics():
            for _ in range(3): cdp.send("HeapProfiler.collectGarbage")
            m = {x["name"]: x["value"] for x in cdp.send("Performance.getMetrics")["metrics"]}
            return int(m["Nodes"]), int(m["JSEventListeners"])
        def flat():
            pg.evaluate(CYC, 5); n0, l0 = metrics()
            pg.evaluate(CYC, 30); n1, l1 = metrics(); print(f"nodes {n0} -> {n1}, listeners {l0} -> {l1}")
            assert n1 - n0 < 300 and l1 - l0 < 60, f"30 more zoom-unzoom cycles: nodes {n0} -> {n1}, listeners {l0} -> {l1}"
        step("30 zoom-unzoom cycles: DOM nodes and listeners stay flat", flat)
        pg.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
