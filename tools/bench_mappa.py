"""Speed of the gestures of the RT x m/z map on the built site (Rust/WASM engine, Chromium headless, synthetic files that scale up to HRMS).

    python3 tools/bench_mappa.py [CASES] [--site DIR] [--wd DIR] [--json]

CASES (default `lr,hrc`): `lr` = 3200-like Full Scan (1500 scans x 500 centroids), `hrc` = Orbitrap Full Scan in centroids (3000 x 3000),
`hrc_big` (6000 x 5000), `hrp` = Orbitrap in profile (400 x 50 000 points), `hrp_big` (600 x 150 000); `NAME:SCANSxPEAKS` sets the size
(e.g. `hrp:300x100000`). Two files per case (A and B: the change of the active file). Without `--site` the site is built in WD/site with
`build_site.py` and the engine .wasm (cargo + wasm-bindgen, as pages.yml). Prints one table per case: time of the first
drawing, wheel zoom, pan, zoom on RT only (the whole m/z band is read), deep zoom, change of file, sight sweep, frame times, memory.
Every number is measured in the page (performance.now()); nothing of the product is changed: the hooks wrap global functions."""
import argparse, base64, json, os, shutil, statistics, subprocess, sys, tempfile, time
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402

CASES = {"lr": ("lr", 1500, 500), "hrc": ("hrc", 3000, 3000), "hrc_big": ("hrc", 6000, 5000), "hrp": ("hrp", 400, 50_000), "hrp_big": ("hrp", 600, 150_000)}


# ---------------------------------------------------------------- synthetic files, written spectrum by spectrum (never the whole file in memory)
def _arr(acc, name, a, bits):
    b = base64.b64encode(np.asarray(a, "<f8" if bits == 64 else "<f4").tobytes()).decode()
    return (f'<binaryDataArray encodedLength="{len(b)}">{ds._cvp("MS:1000523" if bits == 64 else "MS:1000521", f"{bits}-bit float")}'
            f'{ds._cvp("MS:1000576", "no compression")}{ds._cvp(acc, name)}<binary>{b}</binary></binaryDataArray>')


def write_file(path, kind, nscan, npk, seed):
    """`lr`: unit-resolution centroids 80-600; `hrc`: Orbitrap centroids 100-1000 (200 eluting ions + noise); `hrp`: Orbitrap profile, `npk`
    points on a regular m/z grid 100-1000 with Gaussian peaks of the same ions. Numbers invented."""
    rng = np.random.default_rng(seed); hr = kind != "lr"
    ions = np.sort(rng.uniform(110, 990, 200)); top = rng.uniform(5.0, 20.0, 200); amp = rng.lognormal(np.log(2e6), 1.0, 200)
    grid = np.linspace(100, 1000, npk) if kind == "hrp" else None
    filt = "FTMS + p ESI Full ms [100.0000-1000.0000]"
    with open(path, "w", encoding="utf-8") as f:
        f.write(ds._hr_head("exploris", False, Path(path).stem) if hr else '<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="b">')
        f.write(f'<spectrumList count="{nscan}">')
        for i, rt in enumerate(np.linspace(0.3, 25.0, nscan)):
            el = amp * np.exp(-0.5 * ((rt - top) / 0.12) ** 2)
            if kind == "lr":
                mz = np.sort(rng.uniform(80, 600, npk)); it = rng.lognormal(np.log(3e3), 0.8, npk)
            elif kind == "hrc":
                on = el > 1e3; nn = max(0, npk - int(on.sum()))
                mz = np.concatenate([rng.uniform(100, 1000, nn), ions[on] * (1 + rng.normal(0, 1e-6, int(on.sum())))])
                it = np.concatenate([rng.lognormal(np.log(3e3), 0.6, nn), el[on]]); o = np.argsort(mz); mz, it = mz[o], it[o]
            else:
                mz = grid; it = rng.lognormal(np.log(2e2), 0.5, npk)
                for m, a in zip(ions[el > 1e3], el[el > 1e3]):
                    j = np.searchsorted(grid, m); lo, hi = max(0, j - 6), min(npk, j + 6)
                    it[lo:hi] += a * np.exp(-0.5 * ((grid[lo:hi] - m) / (m / 120000)) ** 2)
            b = int(np.argmax(it))
            sc = (ds._cvp("MS:1000016", "scan start time", f"{rt:.6f}", ' unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"')
                  + (ds._cvp("MS:1000800", "mass resolving power", 120000) + ds._cvp("MS:1000512", "filter string", filt) if hr else ""))
            f.write(f'<spectrum index="{i}" id="controllerType=0 controllerNumber=1 scan={i + 1}" defaultArrayLength="{len(mz)}">'
                    + ds._cvp("MS:1000579", "MS1 spectrum") + ds._cvp("MS:1000511", "ms level", 1) + ds._cvp("MS:1000130", "positive scan")
                    + ds._cvp("MS:1000128" if kind == "hrp" else "MS:1000127", "profile spectrum" if kind == "hrp" else "centroid spectrum")
                    + ds._cvp("MS:1000504", "base peak m/z", f"{mz[b]:.6f}", ds._U_MZ) + ds._cvp("MS:1000505", "base peak intensity", f"{it[b]:.6g}")
                    + ds._cvp("MS:1000285", "total ion current", f"{float(it.sum()):.6g}")
                    + f'<scanList count="1">{ds._cvp("MS:1000795", "no combination")}<scan>{sc}</scan></scanList>'
                    + '<binaryDataArrayList count="2">' + _arr("MS:1000514", "m/z array", mz, 64) + _arr("MS:1000515", "intensity array", it, 32)
                    + "</binaryDataArrayList></spectrum>")
        f.write("</spectrumList></run></mzML>")


# ---------------------------------------------------------------- the site
def build_site(out):
    subprocess.run([sys.executable, str(ROOT / "tools" / "build_site.py"), "--out", str(out)], check=True, cwd=ROOT, stdout=subprocess.DEVNULL)
    w = out / "static" / "wasm"
    subprocess.run(["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "-p", "mzlab-wasm"], cwd=ROOT, check=True)
    subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(w), "target/wasm32-unknown-unknown/release/mzlab_wasm.wasm"], cwd=ROOT, check=True)


# the hooks: they wrap global functions of the page (drawMap, mapImage, MAPPA.zoomImg, fetch, Worker) and record when each ends
INIT = """(() => {
  const B = window.__B = { f: [], d: [], z: [], img: [], wmem: 0, fr: null, lt: [] };
  const W = window.Worker;
  window.Worker = class extends W { constructor(u, o) { super(u, o); this.addEventListener('message', e => { const m = e.data; if (m && m.mem) B.wmem = Math.max(B.wmem, m.mem); if (m && m.rd) B.rd = m.rd; }); } };
  const F = window.fetch; let wrapped = false;
  const wrap = () => { if (wrapped || !window.drawMap || !window.MAPPA) return; wrapped = true;
    const f0 = window.fetch;
    window.fetch = function (u, ...a) { const s = String(u), t0 = performance.now(), r = { u: s, t0 };
      if (/api\\/(map|spectra|xic)/.test(s)) B.f.push(r);
      return f0.call(this, u, ...a).then(x => { r.t1 = performance.now(); return x; }); };
    const d0 = window.drawMap; window.drawMap = async function (p) { const t0 = performance.now(); try { return await d0(p); } finally { B.d.push({ t0, t1: performance.now() }); } };
    const m0 = window.mapImage; window.mapImage = function (...a) { const t0 = performance.now(), r = m0.apply(this, a); B.img.push({ t0, ms: performance.now() - t0, hit: performance.now() - t0 < 0.05 }); return r; };
    const z0 = MAPPA.zoomImg; MAPPA.zoomImg = function (...a) { const r = z0.apply(this, a); B.z.push({ t: performance.now(), sharp: r }); return r; };
  };
  B.wrap = wrap;
  B.frames = on => { if (on) { B.fr = []; B.lt = []; const t = a => { if (B.fr) { B.fr.push(a); requestAnimationFrame(t); } }; requestAnimationFrame(t);
      try { B.po = new PerformanceObserver(l => l.getEntries().forEach(e => B.lt.push(e.duration))); B.po.observe({ entryTypes: ['longtask'] }); } catch (_) {} return 0; }
    const fr = B.fr || []; B.fr = null; if (B.po) B.po.disconnect(); const g = []; for (let i = 1; i < fr.length; i++) g.push(fr[i] - fr[i - 1]); return { g, lt: B.lt }; };
})();"""
M = "E.panels.find(q=>q.type==='map')"


def pct(v, q):
    v = sorted(v); return round(v[min(len(v) - 1, int(len(v) * q))], 1) if v else None


def rss_mb():
    """Resident memory of every Chromium process (Linux /proc), MB."""
    t = 0
    for d in Path("/proc").iterdir():
        if not d.name.isdigit(): continue
        try:
            if b"chrom" not in (d / "cmdline").read_bytes()[:300].lower(): continue
            for ln in (d / "status").read_text().splitlines():
                if ln.startswith("VmRSS:"): t += int(ln.split()[1])
        except OSError: pass
    return round(t / 1024)


def wait(pg, js, timeout=600000):
    """poll an expression (the CSP of the page refuses the eval of Playwright's wait_for_function)"""
    t = time.time()
    while not pg.evaluate(js):
        if (time.time() - t) * 1000 > timeout: raise TimeoutError(js[:80])
        time.sleep(0.03)


class Case:
    """One case: two files opened in a fresh page, then the gestures one after the other; every result goes in `out`."""
    def __init__(s, pg):
        s.pg, s.out = pg, {}

    def ev(s, js): return s.pg.evaluate(js)
    def mark(s): return s.ev("performance.now()")
    def since(s, t, what): return s.ev(f"__B.{what}.filter(r => (r.t0 ?? r.t) >= {t})")

    def quiet(s, ms=400):                                                  # no map request in flight and no drawing for `ms`
        wait(s.pg, f"""(() => {{ const B = __B, n = performance.now(); return B.f.every(r => r.t1) && (!B.d.length || n - B.d[B.d.length - 1].t1 > {ms}) && (!B.f.length || n - B.f[B.f.length - 1].t1 > {ms}); }})()""")

    def open(s, port, name, kind, nscan, npk, wd):
        files = []
        for tag, seed in (("A", 11), ("B", 12)):
            f = wd / f"bench_{name}_{nscan}x{npk}_{tag}-t{0 if tag == 'A' else 10}.mzML"
            if not f.exists():
                t = time.time(); write_file(f, kind, nscan + (tag == "B"), npk, seed)     # B one scan longer: never taken for a copy of A; print(f"  file {f.name}: {f.stat().st_size / 1e6:.0f} MB in {time.time() - t:.0f} s", file=sys.stderr, flush=True)
            files.append(f)
        s.out.update({"case": name, "scans": nscan, "points_per_scan": npk, "points_total_M": round(nscan * npk / 1e6, 1), "mzML_MB": round(files[0].stat().st_size / 1e6)})
        pg = s.pg; pg.goto(f"http://127.0.0.1:{port}/?riparti=1"); pg.wait_for_selector("#drop", timeout=120000)
        wait(pg, "window.MZLAB_LR !== undefined", timeout=120000)
        t0 = time.time(); pg.set_input_files("#pick", [str(f) for f in files])
        wait(pg, "document.querySelectorAll('#flist input[data-k=use]').length >= 2", timeout=1800000)
        pg.click("#opbtn"); wait(pg, "typeof E!=='undefined' && E.files.length >= 2 && E.pending.length === 0", timeout=1800000)
        s.out["open_s"] = round(time.time() - t0, 1)
        s.ev("__B.wrap()")

    def first(s):
        """from the new panel to the first map drawn; then full screen and the split of one whole-map answer (bytes, JSON.parse, base64, colour pass)"""
        t = s.mark(); s.ev("reveal(addPanel('map',{k:E.files[0].k}))")
        wait(s.pg, "typeof E!=='undefined' && E.panels.some(q => q.type === 'map' && q._a)", timeout=1800000); s.quiet()
        d = s.since(t, "d"); fm = [r for r in s.since(t, "f") if "api/map" in r["u"]]
        s.out["first_draw_ms"] = round(d[0]["t1"] - t) if d else None
        s.out["first_map_request_ms"] = round(fm[0]["t1"] - fm[0]["t0"]) if fm else None
        s.out["first_read"] = s.ev("__B.rd || null")            # time and bytes inside the index readers of the worker (cumulative since the open of the files)
        s.out["first_mapImage_ms"] = round(max([r["ms"] for r in s.since(t, "img")] or [0]), 1)
        s.ev(M + ".cv.focus()"); s.pg.keyboard.press("f"); s.pg.wait_for_timeout(800); s.quiet()
        s.c = c = s.ev(f"(()=>{{const r={M}.cv.getBoundingClientRect();return [r.left,r.top,r.width,r.height]}})()")
        s.cx, s.cy = c[0] + c[2] * 0.45, c[1] + c[3] * 0.5
        s.out["whole_split"] = s.ev(f"""(async () => {{ const p = {M}, f = p._a.f, u = `api/map?k=${{f.k}}&level=${{f.lv}}`;
          let t = performance.now(); const tx = await (await fetch(u)).text(); const a = performance.now() - t;
          t = performance.now(); const j = JSON.parse(tx); const b = performance.now() - t;
          t = performance.now(); const bin = atob(j.data), q = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) q[i] = bin.charCodeAt(i); const m = new Float32Array(q.buffer); const c = performance.now() - t;
          t = performance.now(); p._img = null; mapImage(p, {{...j, m, k: f.k}}, null, p.scale || 'lin', 'abs'); const d = performance.now() - t;
          return {{ cells: j.nrt * j.nmz, nrt: j.nrt, nmz: j.nmz, json_kB: Math.round(tx.length / 1024), request_ms: +a.toFixed(1), parse_ms: +b.toFixed(1), b64_ms: +c.toFixed(1), colour_ms: +d.toFixed(1) }}; }})()""")
        s.ev(f"(() => {{ const p = {M}; p._img = null; draw(p); }})()"); s.quiet()

    def gesture(s, label, act):
        """frames during the gesture, then the time from its end to the sharp region (zoomImg true) and the cost of the region request"""
        rd0 = s.ev("(__B.rd || {ms: 0}).ms"); s.ev("__B.frames(true)"); t = s.mark(); act(); te = s.mark(); fr = s.ev("__B.frames(false)")
        try: wait(s.pg, f"__B.z.some(r => r.t > {te} && r.sharp)")
        except TimeoutError: pass
        s.quiet()
        z = [r for r in s.since(te, "z") if r["sharp"]]; fq = [r for r in s.since(t, "f") if "rt0=" in r["u"]]
        g = fr["g"]; dd = [r["t1"] - r["t0"] for r in s.since(t, "d") if r["t0"] <= te]
        s.out[label] = {"gesture_ms": round(te - t), "frame_p50": pct(g, .5), "frame_p95": pct(g, .95), "frame_max": pct(g, 1.0), "long_tasks": len(fr["lt"]),
                        "draws": len(dd), "draw_ms_p50": pct(dd, .5), "draw_ms_max": pct(dd, 1.0), "to_sharp_ms": round(z[0]["t"] - te) if z else None,
                        "region_requests": len(fq), "region_request_ms": pct([r["t1"] - r["t0"] for r in fq if r.get("t1")], .5),
                        "read_ms": round(s.ev("(__B.rd || {ms: 0}).ms") - rd0)}

    def wheel(s, n, dy, mods=None):
        pg = s.pg; pg.mouse.move(s.cx, s.cy)
        if mods: pg.keyboard.down(mods)
        for _ in range(n): pg.mouse.wheel(0, dy); pg.wait_for_timeout(16)
        if mods: pg.keyboard.up(mods)

    def pan(s):
        pg = s.pg; pg.mouse.move(s.cx, s.cy); pg.keyboard.down("Shift"); pg.mouse.down()
        for i in range(30): pg.mouse.move(s.cx - 8 * i, s.cy + 2 * i); pg.wait_for_timeout(16)
        pg.mouse.up(); pg.keyboard.up("Shift")

    def whole(s): s.ev(f"MAPPA.whole({M})"); s.quiet()


class Moves(Case):
    """The measured sequences: zoom, pan, file change, sight sweep, many pans."""

    def gestures(s):
        s.gesture("zoom_wheel", lambda: s.wheel(12, -100))
        s.gesture("pan_zoomed", s.pan)
        s.whole(); s.gesture("zoom_rt_only", lambda: s.wheel(10, -100, "Shift"))
        s.whole()                                                          # a window of 2 min x 2 m/z: the zoom of the search for a TP in HR
        s.gesture("zoom_deep", lambda: s.ev(f"(() => {{ const p = {M}, a = p._a, m = (a.x0 + a.x1) / 2, z = (a.y0 + a.y1) / 2; p.zoom = [m - 1, m + 1]; p.zoomY = [z - 1, z + 1]; draw(p); }})()"))
        s.whole()
        for lab, k in (("file_change_cold", 1), ("file_back_warm", 0), ("file_change_warm", 1)):      # the panel shows B (cold), then A, then B again (warm)
            t = s.mark(); s.ev(f"(() => {{ const p = {M}; p.k = E.files[{k}].k; if (E.browse) E.cur = p.k; draw(p); }})()"); s.quiet()
            d = s.since(t, "d"); s.out[lab + "_ms"] = round(d[-1]["t1"] - t) if d else None

    def sight(s):
        """a sweep of 60 pointer moves 50 ms apart: latency of api/spectra (one scan) and frames"""
        pg, c = s.pg, s.c; s.ev(M + ".cv.focus()")
        if not s.ev(M + ".el.classList.contains('mzmir')"): pg.keyboard.press("m")      # full screen turns the sight on already
        pg.wait_for_timeout(400); s.quiet()
        s.ev("__B.frames(true)"); t = s.mark()
        for i in range(60): pg.mouse.move(c[0] + c[2] * 0.15 + i * c[2] * 0.01, s.cy + (i % 5) * 3); pg.wait_for_timeout(50)
        fr = s.ev("__B.frames(false)"); s.quiet(600)
        sp = [r["t1"] - r["t0"] for r in s.since(t, "f") if "api/spectra" in r["u"] and r.get("t1")]
        s.out["sight"] = {"spectra_requests": len(sp), "spectrum_ms_p50": pct(sp, .5), "spectrum_ms_p95": pct(sp, .95), "frame_p50": pct(fr["g"], .5), "frame_p95": pct(fr["g"], .95), "long_tasks": len(fr["lt"])}

    def pans(s):
        """20 pans of a zoomed view, each waiting for its sharp region: entries of the page cache and JS heap kept afterwards; then the memory"""
        heap = "[CACHE.size, Math.round((performance.memory||{}).usedJSHeapSize/1048576)]"
        s.whole(); h0 = s.ev(heap)
        for i in range(20):
            te = s.mark()
            s.ev(f"(() => {{ const p = {M}, a = p._a, w = (a.full[1] - a.full[0]) / 6, x = a.full[0] + w * (0.5 + 4 * {i} / 20); p.zoom = [x, x + w]; p.zoomY = null; draw(p); }})()")
            try: wait(s.pg, f"__B.z.some(r => r.t > {te} && r.sharp)", 120000)
            except TimeoutError: pass
        s.quiet(); h1 = s.ev(heap)
        s.out["pans20"] = {"cache_entries_added": h1[0] - h0[0], "js_heap_added_MB": h1[1] - h0[1]}
        s.out["memory"] = {"chromium_rss_MB": rss_mb(), "js_heap_MB": s.ev("Math.round((performance.memory||{}).usedJSHeapSize/1048576)"), "wasm_peak_MB": s.ev("Math.round(__B.wmem/1048576)")}


def run_case(pg, port, name, kind, nscan, npk, wd):
    c = Moves(pg); c.open(port, name, kind, nscan, npk, wd); c.first(); c.gestures(); c.sight(); c.pans()
    return c.out


def table(o):
    rows = [("points (M) / mzML (MB) / open (s)", f'{o["points_total_M"]} / {o["mzML_MB"]} / {o["open_s"]}'),
            ("first drawing: total / api/map / colour pass (ms)", f'{o["first_draw_ms"]} / {o["first_map_request_ms"]} / {o["first_mapImage_ms"]}')]
    w = o["whole_split"]; rows.append(("whole map: cells / JSON kB / request / parse / base64 / colour (ms)", f'{w["nrt"]}x{w["nmz"]} / {w["json_kB"]} / {w["request_ms"]} / {w["parse_ms"]} / {w["b64_ms"]} / {w["colour_ms"]}'))
    for k in ("zoom_wheel", "pan_zoomed", "zoom_rt_only", "zoom_deep"):
        g = o[k]; rows.append((f"{k}: frame p50/p95/max, long tasks, draws (n, p50, max), to sharp, region (n, ms)",
                               f'{g["frame_p50"]}/{g["frame_p95"]}/{g["frame_max"]}, {g["long_tasks"]}, {g["draws"]}/{g["draw_ms_p50"]}/{g["draw_ms_max"]}, {g["to_sharp_ms"]}, {g["region_requests"]}, {g["region_request_ms"]}'))
    rows.append(("change of file: cold / back warm / warm (ms)", f'{o["file_change_cold_ms"]} / {o["file_back_warm_ms"]} / {o["file_change_warm_ms"]}'))
    s = o["sight"]; rows.append(("sight sweep: spectra n, p50/p95 ms, frame p50/p95, long tasks", f'{s["spectra_requests"]}, {s["spectrum_ms_p50"]}/{s["spectrum_ms_p95"]}, {s["frame_p50"]}/{s["frame_p95"]}, {s["long_tasks"]}'))
    q = o["pans20"]; rows.append(("20 pans: page-cache entries / JS heap kept (MB)", f'{q["cache_entries_added"]} / {q["js_heap_added_MB"]}'))
    m = o["memory"]; rows.append(("memory: Chromium RSS / JS heap / WASM peak (MB)", f'{m["chromium_rss_MB"]} / {m["js_heap_MB"]} / {m["wasm_peak_MB"]}'))
    return f"\n### {o['case']} ({o['scans']} x {o['points_per_scan']})\n| measure | value |\n|---|---|\n" + "\n".join(f"| {a} | {b} |" for a, b in rows)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("cases", nargs="?", default="lr,hrc"); ap.add_argument("--site", type=Path); ap.add_argument("--wd", type=Path)
    ap.add_argument("--json", action="store_true"); ap.add_argument("--port", type=int, default=8871)
    a = ap.parse_args()
    wd = a.wd or Path(tempfile.gettempdir()) / "bench_mappa"; wd.mkdir(parents=True, exist_ok=True)
    site = a.site or wd / "site"
    if not (site / "static" / "wasm" / "mzlab_wasm.js").exists():
        if not (shutil.which("cargo") and shutil.which("wasm-bindgen")): sys.exit("the site needs the engine: cargo and wasm-bindgen (or --site DIR of a built site)")
        build_site(site)
    from playwright.sync_api import sync_playwright
    srv = subprocess.Popen([sys.executable, str(ROOT / "tests_e2e" / "servi.py"), str(a.port), str(site)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    res = []
    try:
        time.sleep(1)
        with sync_playwright() as p:
            for spec in a.cases.split(","):
                nm, _, size = spec.partition(":"); kind, ns, npk = CASES[nm]
                if size: ns, npk = (int(v) for v in size.split("x"))
                prof = wd / f"profile_{nm}"; shutil.rmtree(prof, ignore_errors=True)      # a profile on disk: the quota of a real browser (an incognito context refuses ~1 GB)
                b = ctx = p.chromium.launch_persistent_context(str(prof), executable_path=os.environ.get("MZLAB_CHROMIUM") or None, viewport={"width": 1440, "height": 950})
                ctx.set_default_timeout(600000)
                ctx.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}" + INIT)
                pg = ctx.pages[0] if ctx.pages else ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
                try:
                    o = run_case(pg, a.port, nm, kind, ns, npk, wd); o["page_errors"] = errs[:3]; res.append(o)
                    print(json.dumps(o) if a.json else table(o), flush=True)
                finally: b.close(); shutil.rmtree(prof, ignore_errors=True)
    finally:
        srv.terminate()


if __name__ == "__main__":
    main()
