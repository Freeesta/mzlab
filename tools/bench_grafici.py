"""Cost of the chart panels of the Data view on dense synthetic traces (a measurement, not a test; built site with the engine: tools/prepara_sito.py).

    python3 tools/bench_grafici.py [--rate 1|4] [--only big,many]

big   one SRM file, 4 transitions of 10^6 points (20 min), one shown
many  20 SRM files of 10^5 points, all ticked and overlaid
Per set: draw() time of the whole view and of a zoom on the noisy baseline (median of 8, ms), of a y zoom on the baseline (many also in the waterfall view), then the gestures (Ctrl+wheel x30, Shift+drag x60,
hover x120, window resize x10): draw() calls, frame intervals (median / p95 / max ms) and long tasks (> 50 ms: count / longest). Files in /tmp/mzlab_bench_grafici."""
import sys, os, argparse, statistics
from pathlib import Path
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests_e2e"))
from lib import *          # noqa: E402,F401
import numpy as np         # noqa: E402
import misura_pack as mp   # noqa: E402
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402

WORK = Path("/tmp/mzlab_bench_grafici")
POS = '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
HOOK = r"""(() => { const B = window.__B = { n: 0, fr: [], on: false };
  const d0 = window.draw; window.draw = function () { B.n++; return d0.apply(this, arguments); };
  let last = 0; const tick = t => { if (B.on) { if (last) B.fr.push(t - last); last = t; } else last = 0; FRAME.next(tick); }; FRAME.next(tick);
  B.start = () => { B.n = 0; B.fr = []; B.on = true; }; B.stop = () => { B.on = false; return { n: B.n, fr: B.fr }; }; })(); 0"""
P = "E.panels.find(q => q._a && q.type !== 'spec')"


def srm_file(path, n, ntr, seed):
    """SRM chromatograms of n points: baseline with noise, one Gaussian peak per transition and 6 one-point spikes (they must stay visible zoomed out)"""
    rng = np.random.default_rng(seed); t = np.linspace(0, 20, n); ch = []; tot = np.zeros(n)
    for k in range(ntr):
        y = 200 + rng.normal(0, 30, n).clip(-100) + 1e5 * np.exp(-0.5 * ((t - 5 - 3 * k) / 0.05) ** 2)
        y[rng.choice(n, 6, replace=False)] += 5e4 * (k + 1); tot += y
        ch.append(ds._chrom(k + 2, f"SRM SIC Q1={300 + k}.1 Q3={150 + k}.1 sample=1 period=1 experiment=1 transition={k} ce=20 name=T{k}", t, y, POS))
    ds._write(path, [], [ds._chrom(0, "TIC", t, tot), ds._chrom(1, "BPC", t, tot / ntr)] + ch)


def files(name):
    d = WORK / name; d.mkdir(parents=True, exist_ok=True)
    spec = [("G_MRM-big", 1000000, 4, 3)] if name == "big" else [(f"G_MRM-m{i:02d}", 100000, 2, 20 + i) for i in range(20)]
    out = []
    for stem, n, ntr, seed in spec:
        f = d / f"{stem}.mzML"
        if not f.exists(): srm_file(f, n, ntr, seed)
        out.append(f)
    return out


def frames_stats(fr):
    s = sorted(fr)
    return f"{statistics.median(s):.1f} / {s[int(len(s) * .95)]:.1f} / {s[-1]:.1f}" if s else "–"


def gesture(pg, fn):
    pg.evaluate("__B.start()"); t0 = pg.evaluate("performance.now()"); fn(); frames(pg, 3)
    o = pg.evaluate("__B.stop()"); lt = pg.evaluate("t0 => __M.end(t0)", t0)["lt"]
    return [o["n"], frames_stats(o["fr"]), f"{len(lt)} / {max(lt):.0f}" if lt else "0"]


def draw_ms(pg, zoom):
    v = pg.evaluate(f"""(async () => {{ const p = {P}; p.zoom = {zoom}; const out = []; for (let i = 0; i < 8; i++) {{ const t = performance.now(); await draw(p); out.push(performance.now() - t); }}
        p.zoom = null; await draw(p); return out; }})()""")
    return round(statistics.median(v), 1)


def run_set(p, port, name, rate):
    b, ctx, pg = mp.launch(p, rate); rows = []
    try:
        mp.load_files(pg, port, files(name))
        if name == "many": pg.evaluate("E.files.forEach(f => f.vis = true); syncBrowse(); renderFileList(); redrawAll(); 0"); ready(pg)
        pg.evaluate(HOOK); n = pg.evaluate(f"({P})._a.sr.length")
        rows.append(("draw, whole view", draw_ms(pg, "null"))); rows.append(("draw, zoom on the noisy baseline 9.45-9.62 min", draw_ms(pg, "[9.45, 9.615]")))
        pg.evaluate(f"({P}).zoomY = [0, 300]"); rows.append(("draw, y zoom on the baseline 0-300", draw_ms(pg, "null"))); pg.evaluate(f"({P}).zoomY = null")
        if name == "many": pg.evaluate(f"({P}).mode = 'cas'"); rows.append(("draw, waterfall view", draw_ms(pg, "null"))); pg.evaluate(f"({P}).mode = ''"); ready(pg)
        x, y, w, h = pg.evaluate(f"(() => {{ const r = ({P}).cv.getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; }})()"); cx, cy = x + w / 2, y + h / 2

        def wheel():
            pg.mouse.move(cx, cy); pg.keyboard.down("Control")
            for _ in range(30): pg.mouse.wheel(0, -40)
            pg.keyboard.up("Control")

        def pan():
            pg.mouse.move(cx, cy); pg.keyboard.down("Shift"); pg.mouse.down()
            for i in range(60): pg.mouse.move(cx - i * 4, cy)
            pg.mouse.up(); pg.keyboard.up("Shift")
        rows.append(("Ctrl+wheel x30", *gesture(pg, wheel))); rows.append(("Shift+drag x60 (zoomed)", *gesture(pg, pan)))
        rows.append(("hover x120", *gesture(pg, lambda: [pg.mouse.move(x + 80 + i * (w - 120) / 120, cy) for i in range(120)])))
        pg.evaluate(f"(() => {{ const p = {P}; p.zoom = null; draw(p); }})()"); ready(pg)
        rows.append(("window resize x10", *gesture(pg, lambda: [pg.set_viewport_size({"width": 1500 - 30 * i, "height": 1000}) for i in range(10)])))
    finally: b.close()
    return n, rows


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--rate", type=int, default=1); ap.add_argument("--only", default="big,many")
    a = ap.parse_args(); r = Run(port=8902, wd="/tmp/wd_bench_grafici")
    try:
        with sync_playwright() as p:
            for name in a.only.split(","):
                n, rows = run_set(p, r.port, name, a.rate)
                print(f"\n{name}: {n} trace(s) in the panel, CPU {a.rate}x")
                print("| action | ms or draw() calls | frames ms (median / p95 / max) | long tasks (n / max ms) |\n|---|---|---|---|")
                for row in rows: print("| " + " | ".join(str(c) for c in row) + " |" + (" | |" if len(row) == 2 else ""))
    finally: r.close()


if __name__ == "__main__":
    main()
