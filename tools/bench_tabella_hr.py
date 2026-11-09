"""Cost of a large HR feature table in the «Statistica» panel (a measurement, not a test; built site with the engine: tools/prepara_sito.py).

    python3 tools/bench_tabella_hr.py [--rows 100000]

Two synthetic Orbitrap files and a table of ROWS random ions (m/z 100-900, RT 0.5-12 min, seed 5). Measured in the page: loading the table
(TABELLA.fromText) and the first painting of the panel (longest frame and its script), the Kendrick plot (elements, length of its paths), a click on
a point (KMD.selectRow: XIC, profile and the panel), the «Dati» table of the plot (TDATI.open). Long tasks > 50 ms (count / longest). Files in /tmp/mzlab_bench_tabella."""
import sys, os, argparse
from pathlib import Path
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests_e2e"))
from lib import *          # noqa: E402,F401
import numpy as np         # noqa: E402
import misura_pack as mp   # noqa: E402
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402

WORK = Path("/tmp/mzlab_bench_tabella")
NAMES = ["HR_t0", "HR_t30"]
SEL = "() => { window._sel = KMD.result.rows[5].i; KMD.selectRow(window._sel, document.querySelector('#stat-kmd')); }"
SEL_DONE = "() => !!document.querySelector('#kmd-plot g[data-i=\"' + window._sel + '\"] [stroke-width=\"1.6\"]')"


def table(n):
    rng = np.random.default_rng(5)
    mz, rt, a = rng.uniform(100, 900, n), rng.uniform(0.5, 12, n), rng.lognormal(10, 1, (n, 2))
    return ("row ID,row m/z,row retention time," + ",".join(f"{x} Peak area" for x in NAMES) + "\n"
            + "\n".join(f"{i + 1},{mz[i]:.5f},{rt[i]:.3f},{a[i, 0]:.1f},{a[i, 1]:.1f}" for i in range(n)))


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--rows", type=int, default=100_000)
    a = ap.parse_args(); WORK.mkdir(parents=True, exist_ok=True)
    for i, x in enumerate(NAMES):
        f = WORK / f"{x}.mzML"
        if not f.exists(): ds.hr_series(f, np.random.default_rng(i), [(300.1 + k, 1 + k * 0.5, 0.05, 1e6) for k in range(10)])
    r, out = Run(port=8960, wd="/tmp/wd_bench_tabella"), []
    try:
        with sync_playwright() as p:
            b, ctx, pg = mp.launch(p, 1)
            mp.load_files(pg, r.port, [WORK / f"{x}.mzML" for x in NAMES])
            drawer(pg, "stat"); pg.wait_for_selector("#sb-panel-stat details.stat-sec")
            pg.evaluate("t => { window._T = t; }", table(a.rows))
            t0 = pg.evaluate("performance.now()")
            mp.run_in(pg, "() => TABELLA.fromText(window._T)", "() => true", 600000)
            wait_for_js(pg, "document.querySelector('#kmd-plot')", timeout=600000)
            for _ in range(60): frames(pg, 5)
            res = pg.evaluate("t0 => __M.end(t0)", t0); top = max(res["loaf"], key=lambda x: x["d"], default={"d": 0, "src": ""})
            out.append(("table loaded and panel painted: longest frame, ms (script)", f"{top['d']:.0f} ({top['src']})", ""))
            n = pg.evaluate("[document.querySelector('#kmd-plot').getElementsByTagName('*').length, [...document.querySelectorAll('#kmd-plot path')].reduce((s, p) => s + p.getAttribute('d').length, 0)]")
            out.append(("Kendrick plot: SVG elements; characters of path data", f"{n[0]}; {n[1]}", ""))
            for name, act, until in (("click on a point (XIC, profile, panel)", SEL, SEL_DONE),
                                     ("«Dati» table of the plot", "() => TDATI.open(document.querySelector('#kmd-plot'))", "() => !!document.querySelector('#tdati')")):
                s = mp.run_in(pg, act, until, 600000)
                out.append((name + ", ms", f"{s['ms']:.0f}", f"{s['long_n']} / {s['long_max']:.0f}"))
            out.append(("rows written in the «Dati» table", str(pg.evaluate("document.querySelectorAll('#tdati tbody tr:not(.td-more)').length")), ""))
            b.close()
    finally: r.close()
    print(f"\n{a.rows} rows, 2 HR files, CPU 1x\n| action | value | long tasks (n / max ms) |\n|---|---|---|")
    for row in out: print("| " + " | ".join(row) + " |")


if __name__ == "__main__":
    main()
