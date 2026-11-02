"""Cost of the Data file list when hundreds of files are opened at once (PERF-03). A report, not a test: nothing fails on a number, not in the CI.

    python3 tools/prepara_sito.py
    python3 tests_e2e/misura_lista_molti.py [--ns 20,100,300] [--rates 1,4] [--runs 4] [--json OUT.json]

Synthetic LR files (tools/dati_sintetici.py helpers, 120 scans each) copied N times with distinct names, dropped on the start screen. Per run:
drop (or the answer «Open all» above 50 files) -> last list row present, drop -> loaded (nothing read or opened), drop -> ready (first file shown, first chart drawn, page quiet), the number of
renderFileList calls and their total time (the global is wrapped in the page, the product is not changed), main-thread long tasks (> 50 ms) from the drop to ready.
`--patch FILE.js` adds an init script (A/B of an idea without touching the product)."""
import sys, os, json, time, argparse, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np

sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds

INIT = r"""(() => {
  const V = window.__L = { calls: 0, ms: 0, lt: [], on: false };
  try { new PerformanceObserver(l => { if (V.on) l.getEntries().forEach(e => V.lt.push(e.duration)); }).observe({ entryTypes: ['longtask'] }); } catch (_) {}
  V.wrap = () => { const o = window.renderFileList; if (typeof o !== 'function' || o.__w) return;
    const w = function () { const t = performance.now(); try { return o.apply(this, arguments); } finally { if (V.on) { V.calls++; V.ms += performance.now() - t; } } };
    w.__w = 1; for (const k of Object.keys(o)) w[k] = o[k]; window.renderFileList = w; };
})();"""


def small_files(out: Path, n: int) -> list[str]:
    """n small Full Scan files (120 scans) with distinct names and a few different decay times"""
    out.mkdir(parents=True, exist_ok=True)
    base = out / "_base.mzML"
    rts = np.linspace(0.5, 20.0, 120); rng = np.random.default_rng(5); specs, tic = [], []
    for i, rt in enumerate(rts):
        mz = np.sort(np.concatenate([rng.uniform(120, 480, 45), [364.3, 194.2]])); it = np.concatenate([rng.lognormal(8.3, .9, 45), [2e6 * ds._g(rt, 14.3, .06), 1e6 * ds._g(rt, 14.3, .06)]])
        specs.append(ds._spec(i, rt, mz, it)); tic.append(it.sum())
    ds._write(base, specs, [ds._chrom(0, "TIC", rts, tic)])
    names = []
    for i in range(n):
        p = out / f"S_{i:04d}-t{(i % 8) * 5}.mzML"
        if not p.exists(): shutil.copyfile(base, p)
        names.append(str(p))
    return names


def pct(v, q):
    v = sorted(v); return round(v[min(len(v) - 1, int(len(v) * q))], 1) if v else None


def one_run(p, port, rate, files, patch):
    b = p.chromium.launch(args=["--enable-precise-memory-info"])
    ctx = b.new_context(viewport={"width": 1500, "height": 1000}, locale="it-IT", reduced_motion="reduce"); pg = ctx.new_page()
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.add_init_script(INIT)
    if patch: pg.add_init_script(Path(patch).read_text())
    cdp = ctx.new_cdp_session(pg)
    pg.goto(f"http://127.0.0.1:{port}/"); attendi(pg, "window.MZLAB_LR !== undefined", timeout=240000); pg.wait_for_selector("#pick", state="attached", timeout=240000)
    cdp.send("Emulation.setCPUThrottlingRate", {"rate": rate})
    n = len(files); r = {}
    pg.evaluate("__L.wrap(); __L.on = true; window.__t0 = performance.now(); window.__rows = null; window.__ld = null;"
                f"const f = () => {{ if (window.__rows == null && document.querySelectorAll('#flst .fl[data-k]:not(.up)').length >= {n}) window.__rows = performance.now() - window.__t0; if (window.__rows == null) requestAnimationFrame(f); }}; f();")
    pg.set_input_files("#pick", files)
    if n > 50:         # more than 50 files: the page asks once («Open all» / «Choose…»); the time runs from the answer
        pg.wait_for_selector("#askdlg[open]", timeout=60000); pg.evaluate("window.__t0 = performance.now()"); pg.click("#askok")
    pg.wait_for_function("window.__rows != null", timeout=1800000); r["rows_ms"] = round(pg.evaluate("__rows"))
    loaded(pg, 1800000); r["loaded_ms"] = round(pg.evaluate("performance.now() - __t0"))
    k = pg.evaluate("() => { const f = E.files.filter(f => !f.gone)[0]; return f ? f.k : -1; }")
    pg.evaluate("k => soloFile(E.files[k])", k)
    pg.wait_for_function("(E.panels || []).some(p => p._a)", timeout=1800000); r["first_chart_ms"] = round(pg.evaluate("performance.now() - __t0"))
    ready(pg, 1800000); r["ready_ms"] = round(pg.evaluate("performance.now() - __t0") - 400)       # `ready` waits 400 ms of quiet
    v = pg.evaluate("({ calls: __L.calls, ms: __L.ms, lt: __L.lt })")
    r.update(rfl_calls=v["calls"], rfl_ms=round(v["ms"]), long_n=len([x for x in v["lt"] if x > 50]), long_ms=round(sum(x for x in v["lt"] if x > 50)),
             rows=pg.evaluate("document.querySelectorAll('#flst .fl[data-k]').length"))
    b.close(); return r


def med(rs, k): return pct([x[k] for x in rs], .5)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--ns", default="20,100,300"); ap.add_argument("--rates", default="1,4"); ap.add_argument("--runs", type=int, default=4)
    ap.add_argument("--json"); ap.add_argument("--patch"); ap.add_argument("--port", type=int, default=8981); a = ap.parse_args()
    ns = [int(x) for x in a.ns.split(",")]; rates = [int(x) for x in a.rates.split(",")]
    wd = Path("/tmp/wd_lista_molti"); r = Run(port=a.port, wd=str(wd)); res = {}
    try:
        with sync_playwright() as p:
            for n in ns:
                files = small_files(wd / "dati", n)
                for rate in rates:
                    rs = []
                    for i in range(a.runs):
                        rs.append(one_run(p, r.port, rate, files, a.patch)); print(f"N={n} {rate}x run {i + 1}: {rs[-1]}", flush=True)
                    res[f"{n}_{rate}"] = rs
    finally:
        r.close()
    keys = ("rows_ms", "loaded_ms", "first_chart_ms", "ready_ms", "rfl_calls", "rfl_ms", "long_n", "long_ms")
    print("\n| N | CPU | " + " | ".join(keys) + " | rfl / ready |\n|" + "---|" * (len(keys) + 3))
    for k, rs in res.items():
        n, rate = k.split("_"); m = {x: med(rs, x) for x in keys}
        print(f"| {n} | {rate}× | " + " | ".join(str(m[x]) for x in keys) + f" | {round(100 * m['rfl_ms'] / m['ready_ms'], 1)} % |")
    if a.json: Path(a.json).write_text(json.dumps({"sha": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip(), "res": res}, indent=1))


if __name__ == "__main__": main()
