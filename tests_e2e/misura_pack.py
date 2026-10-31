"""Three measurements for the roadmap, on the built site (Rust/WASM engine, Chromium), synthetic data only. A report, not a test: nothing fails on a number, the script is
not in the CI and not in the normal run (name without e2e_), like misura_macchine.py.

    python3 tools/prepara_sito.py                               # once: site/ with the engine
    python3 tests_e2e/misura_pack.py --runs 3 [--parts hr,export,draw] [--json OUT.json] [--md OUT.md]

hr      15 high-resolution DDA files (synthetic Orbitrap): sending + reading + index, first chart, TIC + 5 XIC of every file, one XIC on all files, reopening (same profile),
        engine memory and browser RSS. Same rows as the table «Indice .mzidx» of docs/agenti/misure.md.
export  worker of «Export all my data» / «Import my data» (esporta-dati-worker.js): time from `new Worker` to the first answer (cold: fresh browser, empty HTTP cache; warm:
        same browser again; app: page with the engine up) and the export / import of 50 MB of OPFS files (read, pack, zip, unzip) through the real esporta-dati.js.
draw    long tasks (> 50 ms) and long animation frames while the heaviest charts are drawn, CPU 1x and 4x: a map at full screen (1500 scans x 500 centroids), TIC and XIC with
        20 files, the MSn graph of 80 ions (SVG, a saved graph). The medians of the runs go to the markdown tables."""
import sys, os, json, time, shutil, subprocess, argparse, threading, statistics
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds  # noqa: E402
import bench_mirino  # noqa: E402

WORK = Path("/tmp/wd_pack_files")
HR_N, LR_N, OPFS_MB = 15, 20, 50
INIT = r"""(() => {
  const V = window.__M = { wmem: 0, lt: [], loaf: [], obs: [] };
  const W = window.Worker;
  window.Worker = class extends W { constructor(u, o) { super(u, o); this.addEventListener('message', e => { const m = e.data; if (m && m.mem) V.wmem = Math.max(V.wmem, m.mem); }); } };
  const add = (type, fn) => { try { const o = new PerformanceObserver(l => l.getEntries().forEach(fn)); o.observe({ type, buffered: false }); V.obs.push([o, fn]); } catch (_) {} };
  add('longtask', e => V.lt.push([e.startTime, e.duration]));
  add('long-animation-frame', e => { const s = (e.scripts || []).slice().sort((a, b) => b.duration - a.duration)[0];
    V.loaf.push({ t: e.startTime, d: e.duration, b: e.blockingDuration || 0, src: s ? (String(s.sourceURL || '').split('/').pop() + ':' + (s.sourceFunctionName || s.invoker || '')) : '' }); });
  // the long tasks and long frames that started inside the window from `t0` to two painted frames after now
  V.end = t0 => new Promise(ok => requestAnimationFrame(() => requestAnimationFrame(() => { V.obs.forEach(([o, fn]) => o.takeRecords().forEach(fn)); const t2 = performance.now();
    ok({ ms: t2 - t0, lt: V.lt.filter(x => x[0] + x[1] >= t0 && x[0] <= t2).map(x => x[1]), loaf: V.loaf.filter(x => x.t + x.d >= t0 && x.t <= t2) }); })));
  // run `act`, wait for `until()` and two painted frames
  V.run = (act, until, cap) => new Promise(ok => { const t0 = performance.now(); act();
    const poll = () => { if (until()) V.end(t0).then(ok); else if (performance.now() - t0 > (cap || 120000)) ok(null); else requestAnimationFrame(poll); }; poll(); });
})();"""
PREFS = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
GETJ = """async u => { const r = await fetch(u); if (!r.ok) throw new Error(u + ' ' + r.status); return r.json(); }"""


def med(v):
    v = [x for x in v if x is not None]
    return round(statistics.median(v), 2) if v else None


def rss_mb():
    """Resident memory (MB) of every Chromium process (/proc, Linux); only the browser of this script is running"""
    total = 0
    for d in Path("/proc").iterdir():
        if d.name.isdigit():
            try:
                if b"chrom" in (d / "cmdline").read_bytes()[:400].lower(): total += int((d / "statm").read_text().split()[1]) * 4096
            except OSError: pass
    return round(total / 2**20, 1)


class Rss(threading.Thread):
    """Peak of the resident memory of every Chromium process, sampled every 0.25 s"""
    def __init__(s): super().__init__(daemon=True); s.peak, s.halt = 0.0, threading.Event()
    def run(s):
        while not s.halt.wait(0.25): s.peak = max(s.peak, rss_mb())


def launch(p, rate=1):
    b = p.chromium.launch(args=["--enable-precise-memory-info"])
    ctx = b.new_context(viewport={"width": 1500, "height": 1000}, locale="it-IT", reduced_motion="reduce")
    ctx.add_init_script(PREFS); ctx.add_init_script(INIT)
    pg = ctx.new_page()
    if rate != 1: ctx.new_cdp_session(pg).send("Emulation.setCPUThrottlingRate", {"rate": rate})
    return b, ctx, pg


def start_page(pg, port, path=""):
    pg.goto(f"http://127.0.0.1:{port}/{path}"); attendi(pg, "window.MZLAB_LR !== undefined", timeout=240000); pg.wait_for_selector("#pick", state="attached", timeout=240000)


# ---------------------------------------------------------------- hr
def hr_files(n):
    d = WORK / "hr"; d.mkdir(parents=True, exist_ok=True)
    out = []
    for i in range(n):
        f = d / f"HR_DDA-Exploris-t{i * 5}.mzML"
        if not f.exists(): ds.hr_dda(f, np.random.default_rng(100 + i), "exploris")
        out.append(f)
    return out


def part_hr(p, r, n):
    files = hr_files(n); size = sum(f.stat().st_size for f in files) / 1e6
    b, ctx, pg = launch(p); rss = Rss(); rss.start(); o = {"files": n, "file_mb": round(size, 1)}
    try:
        t = time.time(); start_page(pg, r.port); o["engine_start_s"] = round(time.time() - t, 2)
        t = time.time(); pg.set_input_files("#pick", [str(f) for f in files])
        pg.wait_for_function(f"document.querySelectorAll('#flist input[data-k=use]').length>={n}", timeout=600000); o["send_listed_s"] = round(time.time() - t, 2)
        pg.click("#opbtn"); pg.wait_for_function(f"typeof E !== 'undefined' && E.files.length >= {n}", timeout=1200000); ready(pg, timeout=1200000)
        o["open_s"] = round(time.time() - t, 2)                                                  # sending + reading + index, files listed and the page quiet
        o["entries"] = pg.evaluate("E.files.length")
        t = time.time(); pg.evaluate("document.querySelector('#flist .fl, .fl')?.click()")
        pg.wait_for_function("E.panels.some(p => p._a)", timeout=600000); ready(pg, timeout=600000); o["first_draw_s"] = round(time.time() - t - 0.4, 2)
        ks = pg.evaluate("E.files.map(f => f.k)")
        t = time.time()
        for k in ks:
            pg.evaluate(GETJ, f"api/chrom?k={k}&kind=tic&level=1")
            for mz in (305.0702, 229.6, 346.1218, 412.1, 505.2005): pg.evaluate(GETJ, f"api/xic?k={k}&mz={mz}&tol=0.01&level=1")
        o["tic_xic_all_s"] = round(time.time() - t, 2)
        ms = pg.evaluate("""async ([ks, mzs]) => { const out = [], pts = []; for (const mz of mzs) { const t = performance.now();
            const r = await Promise.all(ks.map(k => fetch(`api/xic?k=${k}&mz=${mz}&tol=0.01&level=1`).then(x => x.json()))); out.push(performance.now() - t); pts.push(JSON.stringify(r[0]).length); } return [out, pts]; }""",
                         [ks, [200 + 7.3 * i for i in range(12)]])
        o["xic_all_ms_median"], o["xic_all_ms_max"] = med(ms[0]), round(max(ms[0]), 1); o["xic_payload_bytes"] = ms[1][0]
        sp = []
        for k in ks[:6]:
            t = time.time(); pg.evaluate(GETJ, f"api/chrom?k={k}&kind=tic&level=1"); sp.append((time.time() - t) * 1000)
        o["tic_switch_ms_median"] = med(sp)
        o["engine_heap_mb"] = round(pg.evaluate("__M.wmem") / 1048576, 1); o["miss"] = pg.evaluate("(window.MZLAB_LR_MISS || []).length")
        pg.close()
        t = time.time(); q = ctx.new_page(); start_page(q, r.port)                              # same profile: OPFS keeps the files and indexes
        q.wait_for_function(f"typeof E !== 'undefined' && E.files.length >= {n}", timeout=1200000); o["reopen_listed_s"] = round(time.time() - t, 2)
        q.evaluate("document.querySelector('#flist .fl, .fl')?.click()"); q.wait_for_function("E.panels.some(p => p._a)", timeout=600000)
        o["reopen_first_draw_s"] = round(time.time() - t, 2)
    finally:
        rss.halt.set(); o["rss_peak_mb"] = round(rss.peak, 1); b.close()
    return o


# ---------------------------------------------------------------- export
PACK = """async ([path, bytes]) => { const m = await import(path); const e = [{ name: 'a.json', bytes: new TextEncoder().encode('{}') }];
  const t = performance.now(); await m.work({ op: 'zip', packed: m.pack(e).buffer }); return performance.now() - t; }"""
BIG = """async ([path, mb]) => { const root = await navigator.storage.getDirectory(), d = await root.getDirectoryHandle('librerie', { create: true });
  const chunk = new Uint8Array(1 << 20); let s = 12345; for (let i = 0; i < chunk.length; i++) { s = (s * 1103515245 + 12345) & 0x7fffffff; chunk[i] = s >> 16; }
  for (let i = 0; i < 5; i++) { const w = await (await d.getFileHandle(`bench${i}.mzlib`, { create: true })).createWritable(); for (let j = 0; j < mb / 5; j++) await w.write(chunk); await w.close(); }
  const m = await import(path), lap = {}; let t = performance.now(); const tick = k => { const n = performance.now(); lap[k] = n - t; t = n; };
  const { files, left } = await m.collect(); tick('collect_read_opfs');
  const all = [{ name: 'manifesto.json', bytes: m.manifest(files, left, new Date()) }, ...files]; const packed = m.pack(all); tick('pack');
  const buf = packed.buffer.slice(packed.byteOffset, packed.byteOffset + packed.byteLength); const z = new Uint8Array((await m.work({ op: 'zip', packed: buf }, [buf])).zip); tick('zip_worker');
  const url = URL.createObjectURL(new Blob([z], { type: 'application/zip' })); URL.revokeObjectURL(url); tick('blob');
  const zb = z.buffer.slice(z.byteOffset, z.byteOffset + z.byteLength); const un = await m.work({ op: 'unzip', zip: zb }, [zb]); tick('unzip_worker');
  const tr = performance.now(); const files2 = all.map(e => ({ path: 'librerie/x/' + e.name.split('/').pop(), bytes: e.bytes })).slice(1, 3);
  return { ...lap, zip_mb: z.length / 1048576, entries: all.length, unpacked_mb: un.packed.byteLength / 1048576 }; }"""


def part_export(p, r):
    o = {}
    mod = "/static/esporta-dati.js"
    b, ctx, pg = launch(p)                                                                       # fresh browser: empty HTTP cache
    try:
        pg.goto(f"http://127.0.0.1:{r.port}/static/appname.js")                          # same origin, no engine, no worker of the engine
        pg.wait_for_load_state("load")
        o["worker_cold_ms"] = round(pg.evaluate(PACK, [mod, 0]), 1)
        w = [pg.evaluate(PACK, [mod, 0]) for _ in range(5)]; o["worker_warm_ms"] = med(w)
    finally:
        b.close()
    b, ctx, pg = launch(p)                                                                       # a home connection: 10 Mbit/s, 40 ms; the test server does not compress (the site does)
    try:
        pg.goto(f"http://127.0.0.1:{r.port}/static/appname.js"); pg.wait_for_load_state("load")
        c = ctx.new_cdp_session(pg); c.send("Network.enable"); c.send("Network.emulateNetworkConditions", {"offline": False, "latency": 40, "downloadThroughput": 10e6 / 8, "uploadThroughput": 10e6 / 8})
        o["worker_cold_10mbit_ms"] = round(pg.evaluate(PACK, [mod, 0]), 1)
    finally:
        b.close()
    b, ctx, pg = launch(p)
    try:
        start_page(pg, r.port); ready(pg)                                         # the engine of the page is up
        a = [pg.evaluate(PACK, [mod, 0]) for _ in range(4)]; o["worker_app_first_ms"] = round(a[0], 1); o["worker_app_ms"] = med(a[1:])
        big = pg.evaluate(BIG, [mod, OPFS_MB]); o.update({f"opfs_{k}": round(v, 1) for k, v in big.items()})
        o["opfs_total_export_ms"] = round(sum(big[k] for k in ("collect_read_opfs", "pack", "zip_worker", "blob")), 1)
        o["opfs_mb"] = OPFS_MB
        o["rss_mb"] = rss_mb()
    finally:
        b.close()
    return o


# ---------------------------------------------------------------- draw
def summary(res):
    """one run of `__M.run` -> numbers (ms, count of long tasks, longest, sum, long frames and the script of the longest)"""
    if not res: return {}
    lt = res["lt"]; lf = sorted(res["loaf"], key=lambda x: -x["d"])
    return {"ms": round(res["ms"], 1), "long_n": len(lt), "long_max": round(max(lt), 1) if lt else 0, "long_sum": round(sum(lt), 1),
            "loaf_n": len(lf), "loaf_max": round(lf[0]["d"], 1) if lf else 0, "loaf_src": lf[0]["src"] if lf else ""}


def run_in(pg, act, until, cap=120000):
    """`act` and `until` are page functions given as source text (the page has no eval; Playwright compiles the text)"""
    return summary(pg.evaluate(f"__M.run(({act}), ({until}), {cap})"))


def run_py(pg, act, until):
    """the action is Python (a mouse move, the size of the window): mark, act, wait until `until`, collect"""
    t0 = pg.evaluate("performance.now()"); act(); attendi(pg, until, timeout=120000)
    return summary(pg.evaluate("t0 => __M.end(t0)", t0))


def lr_files(n):
    d = WORK / "lr"; d.mkdir(parents=True, exist_ok=True); out = []
    for i in range(n):
        f = d / f"B_FullMass-t{i * 5}.mzML"
        if not f.exists(): ds.full_scan(f, i * 5, np.random.default_rng(10 + i))
        out.append(f)
    return out


def load_files(pg, port, files):
    start_page(pg, port); stage(pg, files, timeout=300000); pg.click("#opbtn"); attendi(pg, "typeof E !== 'undefined' && E.panels.some(p => p._a)", timeout=600000); ready(pg, timeout=600000)


def draw_tic20(pg, port):
    o = {}; load_files(pg, port, lr_files(LR_N)); o["files"] = pg.evaluate("E.files.length")
    n = pg.evaluate("E.panels.filter(p => p.type === 'chrom').length")
    o["tic_new_panel"] = run_in(pg, "() => document.getElementById('np-chrom').click()", f"() => E.panels.filter(p => p.type === 'chrom').length > {n} && E.panels.filter(p => p.type === 'chrom').every(p => p._a)")
    ready(pg)
    o["tic_resize"] = run_py(pg, lambda: pg.set_viewport_size({"width": 1380, "height": 1000}), "E.panels.every(p => !p.el || p._a) && !E.pending.length")
    ms = pg.evaluate("""(() => { const p = E.panels.find(q => q.type === 'spec' && q.tab === E.tab); return p && p._a ? [...new Set((p._a.lbls || []).filter(l => l.m != null).map(l => l.m))].slice(0, 1) : []; })()""")
    if not ms:                                                                                    # click on the chromatogram to place the line: the linked spectrum gets labels
        c = pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'chrom'), r = p.cv.getBoundingClientRect(); return [r.left + r.width * 0.5, r.top + r.height * 0.4]; })()"); pg.mouse.click(*c); frames(pg, 6); ready(pg)
        ms = pg.evaluate("""(() => { const p = E.panels.find(q => q.type === 'spec' && q.tab === E.tab); return p && p._a ? [...new Set((p._a.lbls || []).filter(l => l.m != null).map(l => l.m))].slice(0, 1) : []; })()""")
    if ms:
        nx = pg.evaluate("E.panels.filter(p => p.type === 'xic').length")
        o["xic_20_files"] = run_in(pg, """() => { const p = E.panels.find(q => q.type === 'spec' && q.tab === E.tab), a = p._a, m = %s;
            AZIONI.esegui("spec.xic", { vista: "spec", p, a, m, has: true, lab: m.toFixed(mzd(p, p.level)), e: { preventDefault() {}, clientX: 0, clientY: 0 }, px: 0, py: 0, pol: a.data[0].f && a.data[0].f.polarity }); }""" % ms[0],
                                  f"() => E.panels.filter(p => p.type === 'xic').length > {nx} && E.panels.filter(p => p.type === 'xic').every(p => p._a)")
    else: o["xic_20_files"] = {"error": "no labelled peak"}
    return o


def draw_map(pg, port):
    o = {}; f = WORK / "big_lr-t0.mzML"
    if not f.exists(): bench_mirino.lr_file(f)
    pg.set_viewport_size({"width": 1920, "height": 1080})
    load_files(pg, port, [f])
    M = "E.panels.find(q => q.type === 'map')"
    o["map_open"] = run_in(pg, "() => document.getElementById('np-map').click()", f"() => !!({M} && {M}._a) && !E.pending.length")
    ready(pg); o["map_open"]["canvas_w"] = pg.evaluate(f"{M}.cv.width"); pg.evaluate(M + ".cv.focus()")
    w0 = o["map_open"]["canvas_w"]                                                                # «f» = full screen: done when the canvas has its new size
    o["map_full_screen"] = run_py(pg, lambda: pg.keyboard.press("f"), f"{M}.cv.width !== {w0} && !E.pending.length"); o["map_full_screen"]["canvas_w"] = pg.evaluate(f"{M}.cv.width")
    ready(pg)
    o["map_resize"] = run_py(pg, lambda: pg.set_viewport_size({"width": 1700, "height": 1000}), f"!E.pending.length && !!{M}._a")
    return o


def draw_msn(pg, port):
    import lib_msngrafo
    start_page(pg, port, "?lang=en")
    pg.evaluate("window.__M.lt.length = 0; window.__M.loaf.length = 0")
    fx = Path(__file__).parent / "fixtures" / "msngrafo_80.json"
    t = pg.evaluate("performance.now()")
    pg.set_input_files("#pick", str(fx)); pg.wait_for_selector("#ig-ro .ig-node", timeout=120000); pg.wait_for_function("document.querySelectorAll('#ig-ro .ig-node').length >= 80", timeout=60000); frames(pg, 4)
    n = pg.evaluate("document.querySelectorAll('#ig-ro .ig-node').length")
    res = pg.evaluate("([t]) => { __M.obs.forEach(([o, fn]) => o.takeRecords().forEach(fn)); return { ms: performance.now() - t, lt: __M.lt.filter(x => x[0] + x[1] >= t).map(x => x[1]), loaf: __M.loaf.filter(x => x.t + x.d >= t) }; }", [t])
    return {"msn_open_graph": {**summary(res), "nodes": n}}


def part_draw(p, r):
    import lib_msngrafo; lib_msngrafo.prepara()
    out = {}
    for rate in (1, 4):
        for name, fn in (("tic20", draw_tic20), ("map", draw_map), ("msn", draw_msn)):
            b, ctx, pg = launch(p, rate); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)[:120]))
            try:
                for k, v in fn(pg, r.port).items(): out[f"{k}@{rate}x"] = v
            except Exception as e: out[f"{name}@{rate}x"] = {"error": str(e).splitlines()[0][:160]}
            finally: b.close()
            if errs: out[f"errors_{name}@{rate}x"] = errs[:3]
    return out


# ---------------------------------------------------------------- aggregate and markdown
def flat(d, pre=""):
    for k, v in d.items():
        if isinstance(v, dict): yield from flat(v, f"{pre}{k}.")
        else: yield f"{pre}{k}", v


def aggregate(runs):
    keys = {}
    for run in runs:
        for part, d in run.items():
            for k, v in flat(d, part + "."): keys.setdefault(k, []).append(v)
    return {k: (med(v) if all(isinstance(x, (int, float)) or x is None for x in v) else (max(set(map(str, v)), key=list(map(str, v)).count))) for k, v in keys.items()}


def tables(m, meta):
    g = lambda k: m.get(k, "–")
    L = [f"### HR opening (Rust engine, {meta['runs']} runs, median)", "| measure | value |", "|---|---|"]
    for k, name in (("hr.open_s", "opening (send + read + index), s"), ("hr.first_draw_s", "first chart after the click on a file, s"), ("hr.tic_xic_all_s", "TIC + 5 XIC of every file (sequential), s"),
                    ("hr.xic_all_ms_median", "one XIC on all files (median), ms"), ("hr.xic_all_ms_max", "one XIC on all files (max), ms"), ("hr.tic_switch_ms_median", "TIC of one file (median), ms"),
                    ("hr.reopen_listed_s", "reopening until the files are listed, s"), ("hr.reopen_first_draw_s", "reopening until the first chart, s"),
                    ("hr.engine_heap_mb", "engine WASM heap, MB"), ("hr.rss_peak_mb", "browser RSS peak, MB")):
        L.append(f"| {name} | {g(k)} |")
    L += ["", f"### Data export / import worker (median of {meta['runs']} runs)", "| measure | value |", "|---|---|"]
    for k, name in (("export.worker_cold_ms", "worker start → first answer, cold (fresh browser, empty HTTP cache), ms"), ("export.worker_cold_10mbit_ms", "same, cold on 10 Mbit/s with 40 ms latency (not compressed, as the test server sends it), ms"), ("export.worker_warm_ms", "same, warm (HTTP cache and compiled module), ms"),
                    ("export.worker_app_first_ms", "same, in the app page with the engine up, first call, ms"), ("export.worker_app_ms", "same, later calls, ms"),
                    ("export.opfs_collect_read_opfs", f"read {OPFS_MB} MB of OPFS files (collect), ms"), ("export.opfs_pack", "pack, ms"), ("export.opfs_zip_worker", "zip in the worker (start included), ms"),
                    ("export.opfs_blob", "Blob and download URL, ms"), ("export.opfs_total_export_ms", "export total, ms"), ("export.opfs_unzip_worker", "import: unzip in the worker, ms"), ("export.opfs_zip_mb", "ZIP size, MB")):
        L.append(f"| {name} | {g(k)} |")
    L += ["", f"### Chart drawing on the main thread (long tasks > 50 ms during the action, median of {meta['runs']} runs)", "| action | CPU | action → painted, ms | long tasks (n) | longest, ms | sum, ms | long frames (n) | longest frame, ms | script of the longest frame |", "|---|---|---|---|---|---|---|---|---|"]
    for a in ("tic_new_panel", "tic_resize", "xic_20_files", "map_open", "map_full_screen", "map_resize", "msn_open_graph"):
        for rate in (1, 4):
            p = f"draw.{a}@{rate}x."
            if p + "ms" in m: L.append(f"| {a} | {rate}x | {g(p + 'ms')} | {g(p + 'long_n')} | {g(p + 'long_max')} | {g(p + 'long_sum')} | {g(p + 'loaf_n')} | {g(p + 'loaf_max')} | {g(p + 'loaf_src')} |")
    return L


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--runs", type=int, default=3); ap.add_argument("--parts", default="hr,export,draw"); ap.add_argument("--hr-files", type=int, default=HR_N)
    ap.add_argument("--json"); ap.add_argument("--md"); ap.add_argument("--port", type=int, default=8981); a = ap.parse_args()
    parts = a.parts.split(","); r = Run(port=a.port, wd="/tmp/wd_pack"); runs = []
    try:
        with sync_playwright() as p:
            for i in range(a.runs):
                one = {}
                for name, fn in (("hr", lambda: part_hr(p, r, a.hr_files)), ("export", lambda: part_export(p, r)), ("draw", lambda: part_draw(p, r))):
                    if name not in parts: continue
                    print(f"== run {i + 1}/{a.runs}: {name}", flush=True)
                    try: one[name] = fn()
                    except Exception as e: one[name] = {"error": str(e).splitlines()[0][:200]}
                    print(json.dumps(one[name])[:900], flush=True)
                runs.append(one)
    finally:
        r.close(); shutil.rmtree("/tmp/wd_pack", ignore_errors=True)
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    meta = {"date": time.strftime("%Y-%m-%d"), "sha": sha, "cpu": os.cpu_count(), "runs": a.runs}
    if a.json: Path(a.json).write_text(json.dumps({"meta": meta, "runs": runs}, indent=1))
    L = tables(aggregate(runs), meta); print("\n".join(L))
    if a.md: Path(a.md).write_text("\n".join(L) + "\n")


if __name__ == "__main__": main()
