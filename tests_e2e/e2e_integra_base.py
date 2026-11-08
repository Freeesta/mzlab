"""The baseline modes of a peak integration (mzlab_core::baseline): the page's QUALITA.peakArea (peak_area of the engine .wasm, peaks-worker.js) and
`mzlab-cli peak-area` give the same bytes for the same trace, from zero and from a line that cuts through the peak (a level and a sloped line);
from zero the area is the trapezoid of the trace. The trace is an XIC of the synthetic low-resolution file, written to a CSV with every digit."""
import sys, os, subprocess, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load
import numpy as np
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_integra_base.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "integra_base_tmp"; D.mkdir(parents=True, exist_ok=True)
LRF = D / "B_FullMass-t30.mzML"
ds.full_scan(LRF, 30, np.random.default_rng(2026))
CLI = ["cargo", "run", "--release", "-q", "-p", "mzlab-cli", "--"]
TRACE = """(()=>{const p=E.panels.find(q=>q.type==='xic'),s=p._a.sr[0];let k=0;s.y.forEach((v,i)=>{if(v>s.y[k])k=i});return {x:Array.from(s.x),y:Array.from(s.y),apex:s.x[k],top:s.y[k]}})()"""

r = Run(port=8983, wd="/tmp/wd_integra_base")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [LRF], 4000, 1)
        pg.evaluate("openXic(null)"); pg.wait_for_selector("#xic-mz"); pg.fill("#xic-mz", "253"); pg.select_option("#xic-file", index=1); pg.click("#xic-go"); ready(pg)
        wait_for_js(pg, "(()=>{const p=E.panels.find(q=>q.type==='xic');return !!(p&&p._a&&p._a.sr&&p._a.sr.length)})()")
        tr = pg.evaluate(TRACE)
        csv = D / "trace.csv"; csv.write_text("".join(f"{a!r},{b!r}\n" for a, b in zip(tr["x"], tr["y"])))
        a, b = tr["apex"] - 0.4, tr["apex"] + 0.4
        cases = [("zero", 1, [], []),
                 ("level", 2, [0, tr["top"] / 2, 0, tr["top"] / 2], ["baseline=line", f"base_y0={tr['top'] / 2!r}"]),
                 ("sloped", 2, [a, tr["top"] * 0.1, b, tr["top"] * 0.7], ["baseline=line", f"base_t0={a!r}", f"base_y0={tr['top'] * 0.1!r}", f"base_t1={b!r}", f"base_y1={tr['top'] * 0.7!r}"])]
        for name, mode, line, sets in cases:
            def same(mode=mode, line=line, sets=sets, name=name):
                page = pg.evaluate("([a,b,m,l])=>{const s=E.panels.find(q=>q.type==='xic')._a.sr[0];return QUALITA.peakArea(s.x,s.y,a,b,m,l)}", [a, b, mode, line])
                assert page, "no answer from the engine"
                args = sum((["--set", x] for x in (sets or ["baseline=zero"])), [])
                cli = subprocess.run(CLI + ["peak-area", str(csv), "--from", repr(a), "--to", repr(b), *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout
                assert page + "\n" == cli, (page, cli)
                v = json.loads(page)
                if name == "zero":
                    x, y = np.array(tr["x"]), np.array(tr["y"]); i0, i1 = int(np.abs(x - a).argmin()), int(np.abs(x - b).argmin())
                    trap = float(np.sum((y[i0 + 1:i1 + 1] + y[i0:i1]) / 2 * np.diff(x[i0:i1 + 1]))) * 60
                    assert abs(v["area"] - trap) <= 1e-9 * trap and v["below"] == 0 and v["baseline"] == "zero", (v, trap)
                else:
                    assert v["baseline"] == "line" and v["below"] > 0 and abs(v["area"] + v["below"]) > 0, v
            step(f"{name}: the page and mzlab-cli peak-area give the same bytes", same)
finally:
    r.close()
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
