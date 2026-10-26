"""Speed of the viewer gestures on the built site (Rust/WASM engine, Chromium headless, synthetic files) and, optionally, of the native engine.

    python3 tools/bench_visore.py [CASES] [--site DIR] [--wd DIR] [--reps N] [--mreps N] [--json] [--native] [--g7 DIR] [--only-g2]

CASES (default `lr,hrc`): `lr` 3200-like Full Scan (1500 scans x 500 centroids), `hrc` Orbitrap centroids (3000 x 3000), `hrc_big` (6000 x 5000), `hrp` Orbitrap
profile (400 x 50 000), `lr20` (20 `lr` files), `hr15` (15 `hrc` files); `NAME:SCANSxPEAKS` changes the size, `NAME:SCANSxPEAKSxFILES` also the number of files.
Files are written by `bench_mappa.write_file` (fixed seeds, scan by scan) into WD; without `--site` the site is built in WD/site as pages.yml does.
Gestures (each >= REPS repetitions, default 30; the time runs from the event of the user, captured in the page, to the end of the `draw()` of the panel
that shows the result): G1 click on the TIC -> spectrum; G2 `AZIONI.esegui("spec.xic")` on a labelled peak -> XIC panel, one file; G3 the same with all the
files of the tab visible (multi-file cases); G4 pointer sweep over the spectrum (frames only: the lens does not exist yet); G5 Ctrl+wheel zoom and Shift+drag
pan of the TIC, the spectrum and XIC traces of 3k, 50k and 1M points (served by the bench, see SYN); G6 ArrowRight; M heap of the page and of the engine
before and after MREPS XICs (default 1000). Phases per gesture: request (wall time of the `api/` fetches), engine (`X-Py-Ms`), JSON (`Response.json`), rest = drawing.
`--native` also runs `visore_tempi` on the `.mzidx` of the first file of each case; `--g7 DIR` builds the >= 2 GB file in DIR and times the native engine on it.
Nothing of the product is changed: the hooks wrap global functions of the page (draw, fetch, Response.json, Worker)."""
import argparse, glob, json, os, platform, shutil, signal, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import bench_mappa as bm  # noqa: E402

CASES = {"lr": ("lr", 1500, 500, 1), "hrc": ("hrc", 3000, 3000, 1), "hrc_big": ("hrc", 6000, 5000, 1), "hrp": ("hrp", 400, 50_000, 1),
         "lr20": ("lr", 1500, 500, 20), "hr15": ("hrc", 3000, 3000, 15)}
ONLY_M = False
ONLY_G2 = False
OPEN_MS = 1500000                                              # the opening of the files gives up after this many ms (the case is reported as not measured)
SYN = {"3k": 3_000, "50k": 50_000, "1M": 1_000_000}          # traces served by the bench for the m/z 1001 / 1002 / 1003 (G5)

# the hooks: capture-phase timestamps of the gestures, end of every draw(), every api/ fetch (and the synthetic traces), Response.json, the memory of the worker
INIT = r"""(() => {
  const V = window.__V = { ev: [], dr: [], fe: [], js: [], wmem: 0, wcur: 0, last: 0, rec: false, fr: null, lt: [], syn: {} };
  for (const t of ['pointerup', 'keydown', 'wheel', 'pointermove']) window.addEventListener(t, e => { if (t === 'pointermove' && !V.rec) return; V.last = performance.now(); V.ev.push({ t: V.last, type: t, key: e.key }); }, true);
  const W = window.Worker;
  window.Worker = class extends W { constructor(u, o) { super(u, o); this.addEventListener('message', e => { const m = e.data; if (m && m.mem) { V.wcur = m.mem; V.wmem = Math.max(V.wmem, m.mem); } }); } };
  const j0 = Response.prototype.json; Response.prototype.json = function () { const t0 = performance.now(); return j0.call(this).then(x => { V.js.push({ t0, t1: performance.now() }); return x; }); };
  const rnd = a => () => { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };
  const synth = (n, k, bin) => { const r = rnd(7 + n), rt = new Array(n), y = new Array(n), pk = [4, 9, 13, 18, 22];
    for (let i = 0; i < n; i++) { const x = 0.3 + 24.7 * i / (n - 1); let v = 200 + 80 * r(); for (const c of pk) v += 2e5 * Math.exp(-0.5 * ((x - c) / 0.05) ** 2); rt[i] = +x.toFixed(4); y[i] = +v.toFixed(1); }
    return bin ? binary(k, rt, y) : JSON.stringify({ mz: 0, tol: 0, traces: [{ k, rt, y }] }); };
  // the api/xicbin layout: u32 header length, the header, zeros up to a multiple of 8, rt and y as float64
  const binary = (k, rt, y) => { const h = new TextEncoder().encode(JSON.stringify({ mz: 0, tol: 0, traces: [{ k, n: rt.length }] })), at = (4 + h.length + 7) & ~7, b = new ArrayBuffer(at + 16 * rt.length);
    new DataView(b).setUint32(0, h.length, true); new Uint8Array(b, 4, h.length).set(h); const f = new Float64Array(b, at); f.set(rt, 0); f.set(y, rt.length); return b; };
  let wrapped = false;
  V.wrap = () => { if (wrapped || !window.draw) return; wrapped = true;
    const f0 = window.fetch;
    window.fetch = function (u, ...a) { const s = String(u), t0 = performance.now(), r = { u: s, t0 }, m = /api\/xic(bin)?\?k=(\d+)&mz=([\d.]+)/.exec(s);
      if (/api\//.test(s)) V.fe.push(r); V.last = t0;
      const syn = m && [1001, 1002, 1003].find(z => Math.abs(+m[3] - z) < 0.5);
      const p = syn ? Promise.resolve().then(() => { const n = syn === 1001 ? 3000 : syn === 1002 ? 50000 : 1000000; const key = m[2] + '|' + n + '|' + (m[1] || ''); return new Response(V.syn[key] || (V.syn[key] = synth(n, +m[2], !!m[1])), { status: 200, headers: { 'Content-Type': m[1] ? 'application/octet-stream' : 'application/json', 'X-Py-Ms': '0' } }); }) : f0.call(this, u, ...a);
      return p.then(x => { r.t1 = performance.now(); r.ms = +(x.headers.get('X-Py-Ms') || 0); V.last = r.t1; return x; }, e => { r.t1 = performance.now(); throw e; }); };
    const d0 = window.draw; window.draw = async function (p) { const t0 = performance.now(); try { return await d0(p); } finally { const t1 = performance.now(); V.dr.push({ id: p.id, type: p.type, t0, t1 }); V.last = t1; } };
  };
  V.take = () => { const o = { ev: V.ev, dr: V.dr, fe: V.fe.filter(r => r.t1), js: V.js }; V.ev = []; V.dr = []; V.fe = V.fe.filter(r => !r.t1); V.js = []; return o; };
  V.frames = on => { if (on) { V.fr = []; V.lt = []; const t = a => { if (V.fr) { V.fr.push(a); requestAnimationFrame(t); } }; requestAnimationFrame(t);
      try { V.po = new PerformanceObserver(l => l.getEntries().forEach(e => V.lt.push(e.duration))); V.po.observe({ entryTypes: ['longtask'] }); } catch (_) {} return 0; }
    const fr = V.fr || []; V.fr = null; if (V.po) V.po.disconnect(); const g = []; for (let i = 1; i < fr.length; i++) g.push(fr[i] - fr[i - 1]); return { g, lt: V.lt }; };
})();"""


def chromium_path():
    """MZLAB_CHROMIUM, else the Chromium that Playwright finds, else any chrome under /opt/pw-browsers."""
    c = os.environ.get("MZLAB_CHROMIUM")
    if c: return c
    for g in ("/opt/pw-browsers/chromium-*/chrome-linux/chrome", "/opt/pw-browsers/chromium/chrome-linux/chrome"):
        h = sorted(glob.glob(g))
        if h: return h[-1]
    return None


def machine():
    """CPU, cores, Python and commit for the header of every table."""
    cpu = ""
    try: cpu = next(l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name"))
    except (OSError, StopIteration): cpu = platform.processor()
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    return {"cpu": cpu, "cores": os.cpu_count(), "os": platform.platform(), "commit": sha}


def files_of(wd, kind, nscan, npk, nfiles):
    """The files of a case (written once: `lr` and `lr20` share the first one)."""
    out = []
    for i in range(nfiles):
        f = wd / f"visore_{kind}_{nscan}x{npk}_{i:02d}.mzML"
        if not f.exists():
            t = time.time(); bm.write_file(f, kind, nscan, npk, 100 + i)
            print(f"  file {f.name}: {f.stat().st_size / 1e6:.0f} MB in {time.time() - t:.0f} s", file=sys.stderr, flush=True)
        out.append(f)
    return out


def build_site(out):
    """The site and the engine as pages.yml builds them (wasm-opt when it can be installed, as tools/wasm_opt.sh)."""
    bm.build_site(out)
    w = out / "static" / "wasm" / "mzlab_wasm_bg.wasm"
    if shutil.which("npm"):
        env = {**os.environ, "RUNNER_TEMP": str(out.parent / "runner_tmp")}
        subprocess.run(["bash", str(ROOT / "tools" / "wasm_opt.sh"), str(w), "1708032", "engine"], cwd=ROOT, env=env, check=False)


def stats(v):
    return {"n": len(v), "p50": bm.pct(v, .5), "p95": bm.pct(v, .95), "max": bm.pct(v, 1.0)} if v else {"n": 0}


def med(v):
    return bm.pct(v, .5) if v else None


class Base:
    """One case: the files opened in a fresh page, then the gestures one after the other; every result goes in `out`."""
    def __init__(s, pg, reps, mreps):
        s.pg, s.reps, s.mreps, s.out = pg, reps, mreps, {}

    def ev(s, js): return s.pg.evaluate(js)

    def quiet(s, ms=250):
        """no request in flight, nothing drawn or fetched for `ms` and no file pending"""
        bm.wait(s.pg, f"""(() => {{ const V = __V; return V.fe.every(r => r.t1) && (typeof E === 'undefined' || !E.pending || E.pending.length === 0) && performance.now() - V.last > {ms}; }})()""")

    def take(s): return s.ev("__V.take()")

    def open(s, port, name, files):
        pg = s.pg; s.name = name
        pg.goto(f"http://127.0.0.1:{port}/?riparti=1"); pg.wait_for_selector("#drop", timeout=120000)
        bm.wait(pg, "window.MZLAB_LR !== undefined", timeout=120000)
        t0 = time.time(); pg.set_input_files("#pick", [str(f) for f in files])
        bm.wait(pg, f"document.querySelectorAll('#flist input[data-k=use]').length >= {len(files)}", timeout=OPEN_MS)
        pg.click("#opbtn"); bm.wait(pg, f"typeof E!=='undefined' && E.files.length >= {len(files)} && E.pending.length === 0", timeout=OPEN_MS)
        s.out["open_s"] = round(time.time() - t0, 1)
        bm.wait(pg, "E.panels.some(p => p.type === 'chrom' && p._a) && E.panels.some(p => p.type === 'spec' && p._a)", timeout=600000)
        s.ev("__V.wrap()"); s.quiet(600); s.take()
        f = files[0]
        s.out.update({"case": name, "files": len(files), "scans": s.ev("E.files[0].nscan || E.files[0].scans || null"), "mzML_MB": round(f.stat().st_size / 1e6), "hr": bool(s.ev("!!window.HR && HR.isHr(E.files[0], E.files[0].lv)"))})
        s.out["browse"] = s.ev("!!E.browse")

    def panel(s, typ): return f"E.panels.find(p => p.type === '{typ}' && p.tab === E.tab)"

    def centre(s, p_expr):
        """scroll the panel to the middle of the window and give the centre of its canvas"""
        s.ev(f"{p_expr}.el.scrollIntoView({{block: 'center'}})"); s.pg.wait_for_timeout(150)
        return s.ev(f"(() => {{ const r = {p_expr}.cv.getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; }})()")

    def analyse(s, lg, types, pid=None, t_start=None):
        """one repetition: total time from the event (or `t_start`) to the end of the last draw of the wanted panels, with the phases"""
        ev = [e for e in lg["ev"] if e["type"] != "pointermove"] or lg["ev"]
        t0 = t_start if t_start is not None else (ev[0]["t"] if ev else None)
        if t0 is None: return None
        dr = [d for d in lg["dr"] if d["type"] in types and d["t1"] >= t0 and (pid is None or d["id"] == pid)]
        if not dr: return None
        fe = [f for f in lg["fe"] if f["t0"] >= t0]; js = sum(j["t1"] - j["t0"] for j in lg["js"] if j["t0"] >= t0)
        total = max(d["t1"] for d in dr) - t0; req = (max(f["t1"] for f in fe) - min(f["t0"] for f in fe)) if fe else 0.0
        return {"total": total, "req": req, "eng": sum(f["ms"] for f in fe), "json": js, "rest": max(0.0, total - req - js), "n_req": len(fe), "draws": len(dr)}

    def collect(s, label, rows):
        rows = [r for r in rows if r]
        if not rows: s.out[label] = {"n": 0}; return
        o = stats([r["total"] for r in rows])
        o["phases_p50"] = {k: round(med([r[k] for r in rows]), 2) for k in ("req", "eng", "json", "rest")}
        o["requests_p50"] = med([r["n_req"] for r in rows]); o["draws_p50"] = med([r["draws"] for r in rows])
        s.out[label] = o

    # ---- G1: click on the TIC at `reps` places -> the linked spectrum drawn

class Gestures(Base):
    """The click and XIC gestures G1-G4."""

    def g1(s):
        c = s.centre(s.panel("chrom")); rows = []
        pos = s.ev(f"(() => {{ const a = {s.panel('chrom')}._a; return Array.from({{length: {s.reps}}}, (_, i) => a.X(a.x0 + (a.x1 - a.x0) * (0.06 + 0.88 * ((i * 0.381966) % 1)))); }})()")
        s.take()
        for x in pos:
            s.pg.mouse.click(c[0] + x, c[1] + c[3] * 0.4); s.quiet(); rows.append(s.analyse(s.take(), ["spec"]))
        s.collect("G1_click_tic_to_spectrum", rows)

    def xic_reps(s, label, n):
        """AZIONI.esegui('spec.xic') on labelled peaks of spectra taken at different places (the page memoises an XIC per m/z: every repetition is a new one)"""
        c = s.centre(s.panel("chrom")); rows = []; done = 0; i = 0
        while done < n and i < n * 4:
            x = s.ev(f"(() => {{ const a = {s.panel('chrom')}._a; return a.X(a.x0 + (a.x1 - a.x0) * (0.06 + 0.88 * (({i} * 0.381966) % 1))); }})()"); i += 1
            s.pg.mouse.click(c[0] + x, c[1] + c[3] * 0.4); s.quiet()
            ms = s.ev(f"""(() => {{ const p = {s.panel('spec')}; return p && p._a ? [...new Set((p._a.lbls || []).filter(l => l.m != null).map(l => l.m))] : []; }})()""")
            s.take()
            for m in ms[: max(1, min(3, n - done))]:
                s.ev(f"""(() => {{ const p = {s.panel('spec')}, a = p._a, m = {m}; __V.mark = performance.now();
                  AZIONI.esegui("spec.xic", {{ vista: "spec", p, a, m, has: true, lab: m.toFixed(mzd(p, p.level)), e: {{ preventDefault() {{}}, clientX: 0, clientY: 0 }}, px: 0, py: 0, pol: a.data[0].f && a.data[0].f.polarity }}); }})()""")
                t = s.ev("__V.mark"); s.quiet(); lg = s.take()
                rows.append(s.analyse(lg, ["xic"], t_start=t)); done += 1
                s.ev("(() => { const xs = E.panels.filter(p => p.type === 'xic'); xs.forEach(closePanel); })()"); s.quiet(); s.take()
        s.collect(label, rows)

    def g2(s): s.xic_reps("G2_xic_one_file", s.reps)

    def g3(s):
        s.ev("setVisOf(E.files, true)"); s.quiet(600); s.take()
        s.ev("E.browse = false") if s.out.get("browse") else None
        s.xic_reps("G3_xic_all_files", s.reps)
        s.out["G3_files_shown"] = s.ev("shown().length")

    # ---- G4: a sweep of the pointer over the spectrum, 60 moves 30 ms apart (the hover of today); frames and long tasks
    def g4(s):
        c = s.centre(s.panel("spec")); pg = s.pg
        pg.mouse.move(c[0] + c[2] * 0.2, c[1] + c[3] * 0.5); s.quiet(); s.take()
        s.ev("__V.frames(true)")
        for i in range(60): pg.mouse.move(c[0] + c[2] * (0.2 + i * 0.01), c[1] + c[3] * (0.3 + 0.1 * (i % 5))); pg.wait_for_timeout(30)
        fr = s.ev("__V.frames(false)"); s.take()
        s.out["G4_sweep_spectrum"] = {"frame_p50": bm.pct(fr["g"], .5), "frame_p95": bm.pct(fr["g"], .95), "frames": len(fr["g"]), "long_tasks": len(fr["lt"])}

    # ---- G5: Ctrl+wheel zoom and Shift+drag pan, one event at a time

class Viewer(Gestures):
    """Plus the zoom/pan, scan-stepping and memory gestures."""

    def g5_panel(s, label, p_expr, pid):
        c = s.centre(p_expr); pg = s.pg; zin, zout, pan = [], [], []
        pg.mouse.move(c[0] + c[2] * 0.45, c[1] + c[3] * 0.5); s.quiet(); s.take()
        steps = [dy for _ in range(s.reps) for dy in (-100, 100)]            # in, out, in, out: the view never reaches the limit of the zoom
        for dy in steps:
            pg.keyboard.down("Control"); pg.mouse.wheel(0, dy); pg.keyboard.up("Control"); s.quiet(200)
            (zin if dy < 0 else zout).append(s.analyse(s.take(), [s.ev(f"{p_expr}.type")], pid))
        s.ev(f"(() => {{ const p = {p_expr}, a = p._a, w = (a.x1 - a.x0) * 0.3, m = (a.x0 + a.x1) / 2; p.zoom = [m - w, m + w]; draw(p); }})()"); s.quiet(); s.take()
        x0 = c[0] + c[2] * 0.5; pg.mouse.move(x0, c[1] + c[3] * 0.5); pg.keyboard.down("Shift")
        for i in range(s.reps):
            pg.mouse.down(); s.ev("__V.rec = true; __V.take()"); pg.mouse.move(x0 + (60 if i % 2 else -60), c[1] + c[3] * 0.5, steps=1)
            s.quiet(200); lg = s.take(); s.ev("__V.rec = false"); pg.mouse.up(); pg.mouse.move(x0, c[1] + c[3] * 0.5); s.quiet(200); s.take()
            pan.append(s.analyse(lg, [s.ev(f"{p_expr}.type")], pid))
        pg.keyboard.up("Shift")
        s.collect(f"G5_{label}_zoom_in", zin); s.collect(f"G5_{label}_zoom_out", zout); s.collect(f"G5_{label}_pan", pan)

    def g5(s, synthetic=True):
        for typ, lab in (("chrom", "tic"), ("spec", "spectrum")):
            p_expr = s.panel(typ); s.g5_panel(lab, p_expr, s.ev(f"{p_expr}.id"))
        s.out["G5_tic_points"] = s.ev(f"{s.panel('chrom')}._a.sr[0].x.length")
        if not synthetic: return
        for lab, mz in (("3k", 1001), ("50k", 1002), ("1M", 1003)):
            s.take(); t = s.ev(f"(() => {{ __V.mark = performance.now(); const p = xicDirect({mz}, null); __V.pid = p.id; return __V.mark; }})()")
            bm.wait(s.pg, f"E.panels.some(p => p.id === __V.pid && p._a)", timeout=600000); s.quiet(400)
            lg = s.take(); s.out[f"G5_xic{lab}_first_draw"] = s.analyse(lg, ["xic"], t_start=t)
            pid = s.ev("__V.pid"); s.g5_panel(f"xic{lab}", f"E.panels.find(p => p.id === {pid})", pid)
            s.ev(f"closePanel(E.panels.find(p => p.id === {pid}))"); s.quiet(); s.take()

    # ---- G6: ArrowRight -> next scan, spectrum drawn
    def g6(s):
        c = s.centre(s.panel("chrom")); rows = []
        s.pg.mouse.click(c[0] + c[2] * 0.3, c[1] + c[3] * 0.4); s.quiet(); s.take()
        for i in range(s.reps + 1):
            s.pg.keyboard.press("ArrowRight"); s.quiet(); rows.append(s.analyse(s.take(), ["spec"]))
            if i == 0: si0, rows = s.ev(f"{s.panel('spec')}.si"), []        # the first step only starts the walk (the walk is reported from the second)
        s.collect("G6_arrow_next_scan", rows)
        si1 = s.ev(f"{s.panel('spec')}.si"); s.out["G6_scans_per_press"] = round((si1 - si0) / s.reps, 2) if si0 is not None and si1 is not None else None

    # ---- M: the heap of the page and the memory of the engine before and after `mreps` XICs
    def mem(s):
        heap = "(() => { if (window.gc) window.gc(); return Math.round(((performance.memory || {}).usedJSHeapSize || 0) / 1048576 * 10) / 10; })()"
        s.quiet(); s.take(); h0 = s.ev(heap); w0 = s.ev("Math.round(__V.wcur / 1048576)")
        t = time.time()
        s.ev(f"""(async n => {{ for (let i = 0; i < n; i++) {{ const pn = xicDirect(90 + (i * 0.37) % 500, null);
          await new Promise(r => {{ const w = () => (pn._a && __V.fe.every(f => f.t1)) ? r() : setTimeout(w, 4); w(); }}); closePanel(pn); }} }})({s.mreps})""")
        s.quiet(); h1 = s.ev(heap); w1 = s.ev("Math.round(__V.wcur / 1048576)")
        s.out["M_memory"] = {"xics": s.mreps, "run_s": round(time.time() - t, 1), "js_heap_MB_before": h0, "js_heap_MB_after": h1, "engine_MB_before": w0, "engine_MB_after": w1,
                             "engine_peak_MB": s.ev("Math.round(__V.wmem / 1048576)"), "chromium_rss_MB": bm.rss_mb(), "panels_after": s.ev("E.panels.length")}

    # ---- M of the lens (V-5): `mreps` distinct ions previewed one after the other (the 64-entry cache of the lens turns over), growth every quarter
    def mem_lens(s):
        heap = "(async () => { for (let i = 0; i < 4; i++) { if (window.gc) window.gc(); await new Promise(r => setTimeout(r, 50)); } return Math.round(((performance.memory || {}).usedJSHeapSize || 0) / 1048576 * 10) / 10; })()"
        if not s.ev("!!window.XICLENS"): return
        s.ev(f"{s.panel('spec')}.cv.scrollIntoView()"); s.quiet(); s.take()
        h0 = s.ev(heap); w0 = s.ev("Math.round(__V.wcur / 1048576)"); steps = [{"n": 0, "js_heap_MB": h0, "engine_MB": w0}]
        t = time.time(); q = max(1, s.mreps // 4)
        for a in range(0, s.mreps, q):
            s.ev(f"""(async ([a, b]) => {{ const p = {s.panel('spec')}; for (let i = a; i < b; i++) {{ const m = 60 + i + (i > 938 ? 5 : 0); XICLENS.show(p, m);
              await new Promise(r => {{ const w = () => (XICLENS.shown(p) === m && XICLENS.trace(p)) ? r() : setTimeout(w, 2); w(); }}); }} XICLENS.close(p); }})([{a}, {min(a + q, s.mreps)}])""")
            s.quiet(); steps.append({"n": min(a + q, s.mreps), "js_heap_MB": s.ev(heap), "engine_MB": s.ev("Math.round(__V.wcur / 1048576)")})
        s.out["M_lens"] = {"previews": s.mreps, "run_s": round(time.time() - t, 1), "steps": steps, "cache_after": s.ev("XICLENS.cacheSize()"), "engine_peak_MB": s.ev("Math.round(__V.wmem / 1048576)"),
                           "js_heap_growth_MB": round(steps[-1]["js_heap_MB"] - h0, 1), "engine_growth_MB": steps[-1]["engine_MB"] - w0, "chromium_rss_MB": bm.rss_mb()}


def run_case(pg, port, name, files, reps, mreps, one_synthetic):
    v = Viewer(pg, reps, mreps); v.open(port, name, files)
    if ONLY_M: v.mem_lens(); return v.out
    v.g1()
    multi = len(files) > 1
    if multi: v.ev("setVisOf(E.files.slice(1), false)"); v.quiet(600); v.take()
    v.g2()
    if ONLY_G2: return v.out
    if multi: v.g3()
    v.g4(); v.g6(); v.g5(synthetic=one_synthetic)
    if multi: v.ev("setVisOf(E.files.slice(1), false)"); v.quiet(600); v.take()      # M counts the XICs of one file: G3 has its own
    v.mem(); v.mem_lens()
    return v.out


def native(idx_of, json_out):
    """`visore_tempi` on the `.mzidx` of a file (built from the mzML of the case by the same example)."""
    exe = ROOT / "target" / "release" / "examples" / "visore_tempi"
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "mzlab-core", "--example", "visore_tempi"], cwd=ROOT, check=True)
    r = subprocess.run([str(exe), str(idx_of), "200", "--json"], capture_output=True, text=True, check=True)
    return json.loads(r.stdout.strip().splitlines()[-1])


def g7(dir_, scans, peaks):
    """The >= 2 GB mzML of the native check (G7): written, indexed and timed by `visore_tempi`, then the 200 m/z timings on its `.mzidx`."""
    exe = ROOT / "target" / "release" / "examples" / "visore_tempi"
    subprocess.run(["cargo", "build", "--locked", "--release", "-p", "mzlab-core", "--example", "visore_tempi"], cwd=ROOT, check=True)
    dir_.mkdir(parents=True, exist_ok=True)
    gen = subprocess.run([str(exe), "genera", str(dir_), str(scans), str(peaks)], capture_output=True, text=True, check=True).stdout
    res = dict(p.split("=") for line in gen.split() for p in [line] if "=" in p)
    res.update(native(dir_ / "visore.mzidx", None)); return res


def g7_site(pg, port, mzml, reps=100):
    """G7 in the browser: the file is opened as it is (OPFS copy and `.mzidx` built by the engine), then `reps` XICs straight from the engine
    (`api/xic`, LR 0.5 Da and HR 5 ppm, a grid of m/z over 60-1500): request, JSON read and engine time. Reports the time of the opening."""
    pg.goto(f"http://127.0.0.1:{port}/?riparti=1"); pg.wait_for_selector("#drop", timeout=120000)
    bm.wait(pg, "window.MZLAB_LR !== undefined", timeout=120000)
    t0 = time.time(); pg.set_input_files("#pick", [str(mzml)])
    bm.wait(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 1", timeout=OPEN_MS)
    pg.click("#opbtn"); bm.wait(pg, "typeof E!=='undefined' && E.files.length >= 1 && E.pending.length === 0", timeout=OPEN_MS)
    o = {"mzML_MB": round(Path(mzml).stat().st_size / 1e6), "open_s": round(time.time() - t0, 1)}
    for lab, tol in (("LR_0.5Da", "0.5"), ("HR_5ppm", "ppm")):
        r = pg.evaluate("""async ([n, tol]) => { const q = [], b = [];
          for (let i = 0; i < n + 5; i++) { const mz = 60 + (i * 14.37) % 1440, t = performance.now();
            const x = await fetch(`api/xic?k=${E.files[0].k}&mz=${mz}&tol=${tol === 'ppm' ? mz * 5e-6 : 0.5}&level=1`), t1 = performance.now(), tx = await x.text(), t2 = performance.now();
            if (i >= 5) { q.push([t1 - t, t2 - t1, +(x.headers.get('X-Py-Ms') || 0), t2 - t]); b.push(tx.length); } }
          return { q, b }; }""", [reps, tol])
        col = lambda j: [x[j] for x in r["q"]]
        o[lab] = {"n": reps, "request_p50": bm.pct(col(0), .5), "request_p95": bm.pct(col(0), .95), "read_p50": bm.pct(col(1), .5), "engine_p50": bm.pct(col(2), .5),
                  "engine_p95": bm.pct(col(2), .95), "total_p50": bm.pct(col(3), .5), "total_p95": bm.pct(col(3), .95), "bytes_p50": bm.pct(r["b"], .5)}
    o["engine_MB"] = pg.evaluate("Math.round(__V.wmem / 1048576)"); o["lr_miss"] = pg.evaluate("window.MZLAB_LR_MISS || []")
    return o


def table(o):
    rows = [("files / mzML MB (first) / HR / open (s)", f'{o["files"]} / {o["mzML_MB"]} / {o["hr"]} / {o["open_s"]}')]
    for k, v in o.items():
        if not (k[:2] in ("G1", "G2", "G3", "G4", "G5", "G6") and isinstance(v, dict) and "n" in v): continue
        if v.get("n", 0) == 0: rows.append((k, "no measure")); continue
        ph = v.get("phases_p50"); rows.append((k, f'n {v["n"]}, p50 {v["p50"]} ms, p95 {v["p95"]} ms' + (f' · phases p50: request {ph["req"]}, engine {ph["eng"]}, JSON {ph["json"]}, drawing/rest {ph["rest"]}' if ph else "")))
    if "G4_sweep_spectrum" in o:
        g = o["G4_sweep_spectrum"]; rows.append(("G4_sweep_spectrum", f'frames {g["frames"]}, p50 {g["frame_p50"]} ms, p95 {g["frame_p95"]} ms, long tasks {g["long_tasks"]}'))
    if "M_memory" in o:
        m = o["M_memory"]; rows.append(("M_memory", f'{m["xics"]} XICs in {m["run_s"]} s: JS heap {m["js_heap_MB_before"]} -> {m["js_heap_MB_after"]} MB, engine {m["engine_MB_before"]} -> {m["engine_MB_after"]} MB (peak {m["engine_peak_MB"]}), RSS {m["chromium_rss_MB"]} MB'))
    if "M_lens" in o:
        m = o["M_lens"]; rows.append(("M_lens", f'{m["previews"]} previews in {m["run_s"]} s: JS heap {" -> ".join(str(x["js_heap_MB"]) for x in m["steps"])} MB, engine {" -> ".join(str(x["engine_MB"]) for x in m["steps"])} MB (peak {m["engine_peak_MB"]}), cache {m["cache_after"]}, RSS {m["chromium_rss_MB"]} MB'))
    return f"\n### {o['case']}\n| measure | value |\n|---|---|\n" + "\n".join(f"| {a} | {b} |" for a, b in rows)


ROWS = [("G1 click on the TIC -> spectrum", "G1_click_tic_to_spectrum"), ("G2 XIC from the spectrum, one file", "G2_xic_one_file"), ("G3 XIC, all files of the tab", "G3_xic_all_files"),
        ("G5 TIC zoom (wheel step)", "G5_tic_zoom_in"), ("G5 TIC pan", "G5_tic_pan"), ("G5 spectrum zoom", "G5_spectrum_zoom_in"), ("G5 spectrum pan", "G5_spectrum_pan"),
        ("G5 XIC 3k points zoom", "G5_xic3k_zoom_in"), ("G5 XIC 3k points pan", "G5_xic3k_pan"), ("G5 XIC 50k points zoom", "G5_xic50k_zoom_in"), ("G5 XIC 50k points pan", "G5_xic50k_pan"),
        ("G5 XIC 1M points zoom", "G5_xic1M_zoom_in"), ("G5 XIC 1M points pan", "G5_xic1M_pan"), ("G6 ArrowRight -> next spectrum", "G6_arrow_next_scan")]


def md(path):
    """The starting table (gesture x case, `p50 / p95 ms (n)`) from the JSON lines of a run, for docs/agenti/visore-veloce.md."""
    cs = [o for o in (json.loads(l) for l in open(path) if l.startswith("{")) if "case" in o]
    cell = lambda c, k: (f'{c[k]["p50"]} / {c[k]["p95"]} ({c[k]["n"]})' if isinstance(c.get(k), dict) and c[k].get("n") else "–") if "error" not in c else "not measured"
    out = ["| gesture | " + " | ".join(c["case"] for c in cs) + " |", "|---" * (len(cs) + 1) + "|"]
    out += [f"| {lab} | " + " | ".join(cell(c, k) for c in cs) + " |" for lab, k in ROWS]
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("cases", nargs="?", default="lr,hrc"); ap.add_argument("--site", type=Path); ap.add_argument("--wd", type=Path)
    ap.add_argument("--reps", type=int, default=30); ap.add_argument("--mreps", type=int, default=1000); ap.add_argument("--json", action="store_true")
    ap.add_argument("--port", type=int, default=8873); ap.add_argument("--only-g2", action="store_true", help="only G1 and G2 (the XIC panel)"); ap.add_argument("--only-m", action="store_true", help="only the M of the lens (V-5)"); ap.add_argument("--native", action="store_true"); ap.add_argument("--g7", type=Path)
    ap.add_argument("--g7-site", type=Path, help="open this big mzML in the browser and time XICs on its index (G7 on the site)")
    ap.add_argument("--open-min", type=int, default=25, help="minutes the opening of the files of a case may take")
    ap.add_argument("--g7-size", default="80000x6000", help="SCANSxPEAKS of the file of --g7 (80000x6000 = 2.6 GB of mzML)")
    ap.add_argument("--md", type=Path, help="print the starting table from the JSON lines of a previous run and stop")
    a = ap.parse_args(); global OPEN_MS, ONLY_M, ONLY_G2; OPEN_MS = a.open_min * 60000; ONLY_M = a.only_m; ONLY_G2 = a.only_g2
    if a.md: return print(md(a.md))
    wd = a.wd or Path(tempfile.gettempdir()) / "bench_visore"; wd.mkdir(parents=True, exist_ok=True)
    t_all = time.time(); mach = machine(); print(json.dumps({"machine": mach}) if a.json else f"machine: {mach['cpu']} x{mach['cores']}, commit {mach['commit']}", flush=True)
    if a.g7:
        s, p = (int(x) for x in a.g7_size.split("x")); r = g7(a.g7, s, p)
        print(json.dumps({"g7": r}) if a.json else f"\n### G7 native ({a.g7_size})\n{json.dumps(r, indent=1)}", flush=True)
    if a.cases in ("", "none") and not a.g7_site: return
    site = a.site or wd / "site"
    if not (site / "static" / "wasm" / "mzlab_wasm.js").exists():
        if not (shutil.which("cargo") and shutil.which("wasm-bindgen")): sys.exit("the site needs the engine: cargo and wasm-bindgen (or --site DIR of a built site)")
        build_site(site)
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, str(ROOT / "tests_e2e" / "servi.py"), str(a.port), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        time.sleep(1)
        with sync_playwright() as p:
            if a.g7_site:
                prof = wd / "profile_g7"; shutil.rmtree(prof, ignore_errors=True)
                ctx = p.chromium.launch_persistent_context(str(prof), executable_path=chromium_path(), viewport={"width": 1440, "height": 950}, args=["--enable-precise-memory-info"], locale="it-IT")
                ctx.set_default_timeout(OPEN_MS); ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}" + INIT)
                try:
                    pg = ctx.pages[0] if ctx.pages else ctx.new_page()
                    try: r = g7_site(pg, a.port, a.g7_site)
                    except Exception as e: r = {"error": str(e).splitlines()[0][:300]}
                    print(json.dumps({"g7_site": r}) if a.json else f"\n### G7 site\n{json.dumps(r, indent=1)}", flush=True)
                finally: ctx.close(); shutil.rmtree(prof, ignore_errors=True)
            for k, spec in enumerate([c for c in a.cases.split(",") if c not in ("", "none")]):
                nm, _, size = spec.partition(":"); kind, ns, npk, nf = CASES[nm]
                if size:
                    parts = [int(v) for v in size.split("x")]; ns, npk = parts[0], parts[1]; nf = parts[2] if len(parts) > 2 else nf
                files = files_of(wd, kind, ns, npk, nf)
                prof = wd / f"profile_{nm}"; shutil.rmtree(prof, ignore_errors=True)         # a profile on disk: the quota of a real browser (an incognito context refuses ~1 GB)
                ctx = p.chromium.launch_persistent_context(str(prof), executable_path=chromium_path(), viewport={"width": 1440, "height": 950},
                                                           args=["--enable-precise-memory-info", "--js-flags=--expose-gc"], locale="it-IT")
                ctx.set_default_timeout(OPEN_MS)
                ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}" + INIT)
                pg = ctx.pages[0] if ctx.pages else ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
                signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError("the case did not finish within its time cap")))
                signal.alarm(int(2 * OPEN_MS / 1000) + 900)                                  # a hung browser must not stall the whole run
                try:
                    t = time.time(); o = run_case(pg, a.port, nm, files, a.reps, a.mreps, one_synthetic=len(files) == 1)
                    o["page_errors"] = errs[:3]; o["case_s"] = round(time.time() - t); o["lr_miss"] = pg.evaluate("window.MZLAB_LR_MISS || []")
                    if a.native:
                        idx = wd / f"visore_{nm}.mzidx"
                        subprocess.run([str(ROOT / "target" / "release" / "examples" / "visore_tempi"), "indicizza", str(files[0]), str(idx)], check=True, stdout=subprocess.DEVNULL)
                        o["native"] = native(idx, None)
                    print(json.dumps(o) if a.json else table(o), flush=True)
                except Exception as e:                                                       # a case that does not open in time (hrp in the browser) is a result, not a crash
                    o = {"case": nm, "error": str(e).splitlines()[0][:300], "case_s": round(time.time() - t)}; print(json.dumps(o) if a.json else f"\n### {nm}\nnot measured: {o['error']}", flush=True)
                finally: signal.alarm(0); ctx.close(); shutil.rmtree(prof, ignore_errors=True)
    finally:
        srv.terminate()
    print(f"total {time.time() - t_all:.0f} s", file=sys.stderr)


if __name__ == "__main__":
    main()
