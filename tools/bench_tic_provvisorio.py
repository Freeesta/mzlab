"""Gestures on the provisional TIC while the index builds (tic-provvisorio.js): event time and frame time, and drop -> indexed with and without gestures.

    python3 tools/bench_tic_provvisorio.py [--scans N] [--peaks N] [--reps N] [--throttle X] [--json]

Needs the built site with the engine (python3 tools/prepara_sito.py). One synthetic mzML (tools/mzml_msconvert_synthetic.py; default 40500 scans x 3000 peaks, ~490 MB);
the runs alternate `idle` (no gestures) and `gest` (one Ctrl+wheel / drag / key gesture per animation frame from the moment the box shows until it goes), each in a new browser.
Per gesture: `event` = the synchronous time of the handlers, `frame` = from the event to the end of the next frame; `raf` = the intervals of the frames. Medians with min-max."""
import argparse, json, os, statistics as st, sys, tempfile, shutil, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests_e2e")); sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *  # noqa: E402,F401
import mzml_msconvert_synthetic as gen  # noqa: E402

LOOP = """() => new Promise(res => { const out = { ev: [], fr: [], raf: [], n: 0 };
  const look = () => { const b = document.getElementById('ticprov'); if (!b || b.hidden) return false; return b.querySelector('canvas'); };
  const wait = () => { const cv = look(); if (cv) go(cv); else requestAnimationFrame(wait); };
  const go = cv => { const r = cv.getBoundingClientRect(); let last = performance.now(), k = 0;
    const step = () => { const b = look(); if (!b) return res(out);
      const t0 = performance.now(), ph = k++ % 40, x = r.left + r.width * (0.3 + 0.4 * ((k * 7) % 10) / 10);
      if (ph < 12) cv.dispatchEvent(new WheelEvent('wheel', { deltaY: -250, ctrlKey: true, clientX: x, clientY: r.top + 60, bubbles: true, cancelable: true }));
      else if (ph < 20) cv.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }));
      else if (ph < 32) cv.dispatchEvent(new WheelEvent('wheel', { deltaY: 250, ctrlKey: true, clientX: x, clientY: r.top + 60, bubbles: true, cancelable: true }));
      else cv.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true, cancelable: true }));
      const t1 = performance.now(); out.ev.push(t1 - t0); out.n++;
      requestAnimationFrame(() => { const t2 = performance.now(); out.fr.push(t2 - t0); out.raf.push(t2 - last); last = t2; step(); }); };
    step(); };
  wait(); })"""

def pct(a, q):
    a = sorted(a); return a[min(len(a) - 1, int(round(q * (len(a) - 1))))] if a else float("nan")

def one(mode, f, port, throttle):
    r = Run(port=port, wd=f"/tmp/wd_bench_ticprov_{mode}")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            if throttle != 1:
                c = pg.context.new_cdp_session(pg); c.send("Emulation.setCPUThrottlingRate", {"rate": throttle})
            pg.evaluate("() => { window.__loop = " + LOOP + "; window.__res = null; }")
            if mode == "gest": pg.evaluate("() => { window.__loop().then(r => { window.__res = r; }); }")      # waits in the page for the box; not awaited here
            t0 = time.time()
            pg.set_input_files("#pick", [f])
            out = None
            # loading started (UPBUSY is 0 before the files are taken), then the file open and nothing busy: the index is done
            pg.wait_for_function("UPBUSY > 0 || OPENNOW.busy() > 0 || E.files.some(f => !f.gone)", timeout=120000)
            pg.wait_for_function("E.files.some(f => !f.gone) && UPBUSY === 0 && OPENNOW.busy() === 0", timeout=600000)
            dt = time.time() - t0
            if mode == "gest":
                pg.wait_for_function("window.__res !== null", timeout=60000); out = pg.evaluate("window.__res")
            return dt, out
    finally:
        r.close()

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--scans", type=int, default=40500); ap.add_argument("--peaks", type=int, default=3000)
    ap.add_argument("--reps", type=int, default=5); ap.add_argument("--throttle", type=float, default=1.0); ap.add_argument("--json", action="store_true")
    a = ap.parse_args()
    d = tempfile.mkdtemp(prefix="bench_ticprov_"); f = os.path.join(d, "tic_grande.mzML")
    gen.scrivi(f, scans=a.scans, peaks1=a.peaks, peaks2=150)
    mb = os.path.getsize(f) / 1e6
    res = {"idle": [], "gest": []}; ev, fr, raf = [], [], []
    for i in range(a.reps):
        for mode in ("idle", "gest"):
            t, o = one(mode, f, 8870 + i * 2 + (mode == "gest"), a.throttle); res[mode].append(t)
            if o: ev += o["ev"]; fr += o["fr"]; raf += o["raf"]
    shutil.rmtree(d, ignore_errors=True)
    rep = {"mb": round(mb), "throttle": a.throttle, "reps": a.reps, "gestures": len(ev),
           "drop_to_indexed_s": {m: {"median": round(st.median(v), 2), "min": round(min(v), 2), "max": round(max(v), 2)} for m, v in res.items()},
           "event_ms": {"p50": round(pct(ev, .5), 2), "p98": round(pct(ev, .98), 2)}, "frame_ms": {"p50": round(pct(fr, .5), 2), "p98": round(pct(fr, .98), 2)},
           "raf_interval_ms": {"p50": round(pct(raf, .5), 2), "p95": round(pct(raf, .95), 2)}}
    print(json.dumps(rep, indent=None if a.json else 1))

if __name__ == "__main__":
    main()
