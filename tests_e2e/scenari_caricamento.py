"""Scenarios of `misura_caricamento.py --scenarios`: the critical path of the first file and of many files on the built site. Measures, never asserts.

Arms (one run = one fresh persistent Chromium profile, a normal window as the students have it; `warm` = a copy of a profile whose service-worker fill is done):
  uno / dieci   a synthetic file alone vs the same file first in a drop of ten (mzML 53 MB; `.raw` as `raw-uno` / `raw-dieci`)
  rotte, thermo the single drop with the `.wasm` of the routes module (`rotte`) / of Thermo (`thermo`) answered 3 s after the drop by the server, whenever it was asked (the `.js` glue is not held), on a cold profile
  subito, quieto the drop at qqReady + 0 ms vs + 20 s, cold and warm
  interazione   during `dieci`, once the first chart is drawn: 20 Ctrl+wheel zooms, 20 Shift+drags (3 pointermoves each), 5 clicks on the TIC (-> spectrum); `uno+gesti` = the same with nothing else loading
  memoria       `dieci` with measureUserAgentSpecificMemory every 500 ms, and the RSS of the browser's processes (read from /proc)
  copie, raw-copie, copie-mem, cinquanta   the copy gate (lr.js `copiesOf`): `dieci-kK` = ten files with K copies at once (0 = all at once); `cinquanta-kK` = fifty mzML of ~10 MB (480 scans)
Metrics per run, ms from the `change` of the file input: provisional TIC (`#ticprov` drawn), first chart (a panel drawn), all indexed (nothing read or opened any more), start of the copy
(first `mzlab-index` of the first file), main-thread long tasks and server requests between the drop and the first chart, rAF delta p95 and Event Timing p50/p98 from the first chart on,
input -> next frame of wheel and drag, click -> last spectrum draw, memory peaks. CPU throttling is the page's main thread (workers run at full speed)."""
import json, os, shutil, statistics, subprocess, sys, tempfile, threading, time
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
# tests_e2e/lib.py (imported by misura_caricamento.py) replaces wait_for_timeout (pauses end when the page is quiet) and set_input_files (waits for the rows, writes times):
# both would move a measurement, so the real ones are put back
sys.path.insert(0, str(ROOT / "tests_e2e"))
try:
    import lib as _lib
    from playwright.sync_api import Page as _Page
    _Page.wait_for_timeout, _Page.set_input_files = _lib._real_wft, _lib._real_sif
except ImportError: pass


def nap(pg, ms):
    """Real-time pause measured in the page (the measurement needs wall-clock gaps, not a quiet-page wait)."""
    pg.evaluate("ms => new Promise(r => setTimeout(r, ms))", ms)


_WAIT = __import__("threading").Event().wait
sys.path.insert(0, str(ROOT / "tools"))
CHROME = {"executable_path": "/opt/pw-browsers/chromium"} if os.path.isfile("/opt/pw-browsers/chromium") else {}
DELAY_MS = 3000
PROBE = r"""(() => { if (window.__s) return; const T0 = performance.timeOrigin, S = window.__s = { ev: [], lt: [], et: [], fr: [], lat: [], draws: [], mem: [], idx: [] }, now = () => performance.now();
  const ok = () => typeof E !== 'undefined' && typeof UPBUSY !== 'undefined' && typeof OPENNOW !== 'undefined';
  document.addEventListener('change', e => { if (e.target && e.target.id === 'pick' && !S.t0) { S.t0 = now(); S.t0e = T0 + S.t0; } }, true);
  addEventListener('mzlab-index', e => { if (S.t0) S.idx.push([Math.round(now() - S.t0), e.detail.name, e.detail.phase || '', +(+e.detail.frac || 0).toFixed(3)]); });
  try { new PerformanceObserver(l => l.getEntries().forEach(e => { if (S.t0 && e.startTime + e.duration >= S.t0) S.lt.push([Math.round(e.startTime - S.t0), Math.round(e.duration)]); })).observe({ type: 'longtask', buffered: true }); } catch (_) {}
  try { new PerformanceObserver(l => l.getEntries().forEach(e => S.et.push([e.name, Math.round(e.startTime), e.duration]))).observe({ type: 'event', durationThreshold: 16, buffered: true }); } catch (_) {}
  const sinceInput = kind => e => { const t = now(); requestAnimationFrame(() => requestAnimationFrame(() => S.lat.push([kind + (ok() && (UPBUSY > 0 || OPENNOW.busy() > 0) ? ':busy' : ''), now() - t]))); };
  addEventListener('wheel', sinceInput('wheel'), true);
  addEventListener('pointermove', e => { if (e.buttons & 1) sinceInput('drag')(e); }, true);
  addEventListener('click', e => { window.__click = now(); }, true);
  let last = 0; const tick = () => { const t = now(); if (S.tchart && !S.tall) S.fr.push(t - last); last = t; requestAnimationFrame(tick); }; requestAnimationFrame(tick);
  setInterval(() => { if (!window.qqReady || S.tready) { if (!ok() || !S.t0) return; } else S.tready = now();
    if (!S.t0) return; const t = now() - S.t0, tp = document.getElementById('ticprov');
    if (!S.tprov && tp && !tp.hidden && tp.querySelector('canvas') && tp.querySelector('canvas').width > 0) S.tprov = t;
    if (!S.tchart && E.panels.some(p => p._a)) S.tchart = t;
    if (S.tchart && !S.tall && UPBUSY === 0 && OPENNOW.busy() === 0) { S.n = (S.n || 0) + 1; if (S.n >= 2) S.tall = t; } else if (!S.tall) S.n = 0; }, 20);
  S.wrapDraw = () => { const o = window.draw; if (typeof o !== 'function' || o.__w) return; const w = function (p) { try { return o.apply(this, arguments); } finally { if (S.t0) S.draws.push([now(), p && p.type]); } }; w.__w = 1; for (const k of Object.keys(o)) w[k] = o[k]; window.draw = w; };
  // measureUserAgentSpecificMemory resolves seconds after the call, so calls overlap: a sample is [ms of the call, bytes at the answer, JS heap of the page at the call]
  S.memOn = false; S.pend = 0; S.memLoop = () => { const t = setInterval(() => { if (!S.memOn) return clearInterval(t); const at = Math.round(now() - (S.t0 || now())), h = performance.memory ? performance.memory.usedJSHeapSize : 0; S.pend++;
    performance.measureUserAgentSpecificMemory().then(r => S.mem.push([at, r.bytes, h, Math.round(now() - (S.t0 || now()))]), e => { S.memErr = String(e); }).finally(() => S.pend--); }, 500); };
})()"""


class Site:
    """The built site on a thread of this process: the files the arm holds back (a request for them is answered DELAY_MS after the drop, whenever it was asked: a module the
    service worker's fill or the page asked for before the drop is held too) and a log of every request that reaches the network (start, end, bytes); the fill shows too."""
    def __init__(s, site, port):
        s.hold, s.release, s.log, root = (), None, [], str(site)
        me = s
        class H(SimpleHTTPRequestHandler):
            def log_message(self, *a): pass
            def do_GET(self):
                t = time.time() * 1000; path = self.path.split("?")[0]; f = Path(root) / path.lstrip("/")
                row = [t, path, f.stat().st_size if f.is_file() else 0, 0]; me.log.append(row)      # logged on arrival: a held request shows although the run ends before it is answered
                if any(k in path for k in me.hold):
                    for _ in range(800):
                        if me.release is not None and time.time() * 1000 >= me.release: break
                        _WAIT(0.05)
                try: super().do_GET()
                finally: row[3] = time.time() * 1000
        class Srv(ThreadingHTTPServer):
            request_queue_size = 256; daemon_threads = True
            def handle_error(self, request, client_address): pass
        s.srv = Srv(("127.0.0.1", port), partial(H, directory=root)); s.port = port
        threading.Thread(target=s.srv.serve_forever, daemon=True).start()
    def close(s): s.srv.shutdown()


def make_files(wd, raw, n=10, scans=None):
    """`n` synthetic files (ten by default) of ~53 MB: mzML (msconvert style, 2400 scans) or a Thermo `.raw` (thermo_synthetic); copies with other names after the first.
    `scans` makes a smaller mzML (the 50-file batch: 480 scans, ~10 MB)."""
    import mzml_msconvert_synthetic as gen
    wd.mkdir(parents=True, exist_ok=True); ext = "raw" if raw else "mzML"; base = wd / f"S01.{ext}"
    if not base.exists():
        if raw:
            exe = ROOT / "target" / "release" / "examples" / "thermo_synthetic"
            subprocess.run([str(exe), str(base), "--scans", "950", "--peaks", "600"], check=True, capture_output=True)
        else: gen.scrivi(str(base), scans=scans or 2400, peaks1=5000, peaks2=500)
    out = [str(base)]
    for i in range(2, n + 1):
        p = wd / f"S{i:02d}.{ext}"
        if not p.exists(): shutil.copyfile(base, p)
        out.append(str(p))
    return out


def rss_mb(prof):
    """Sum of the resident memory of the browser's processes (those started with this profile directory), MB, from /proc: no browser call."""
    tot = 0
    for d in os.listdir("/proc"):
        if not d.isdigit(): continue
        try:
            if prof.encode() in open(f"/proc/{d}/cmdline", "rb").read(): tot += int(open(f"/proc/{d}/statm").read().split()[1]) * 4096
        except Exception: pass
    return tot / 1e6


class RssSampler(threading.Thread):
    def __init__(s, prof): super().__init__(daemon=True); s.prof, s.on, s.v = prof, True, []
    def run(s):
        while s.on: s.v.append(rss_mb(s.prof)); _WAIT(0.5)


def open_ctx(p, prof, cpu_hint=None, copie=None):
    ctx = p.chromium.launch_persistent_context(prof, viewport={"width": 1500, "height": 1000}, reduced_motion="reduce", **CHROME,
                                               args=["--headless=new", "--enable-precise-memory-info"])
    ctx.set_default_timeout(900000)
    ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    ctx.add_init_script(PROBE)
    if copie is not None: ctx.add_init_script(f"try{{localStorage.setItem('mzlab.dev.copie','{int(copie)}')}}catch(_){{}}")      # the copy gate's switch (lr.js copiesOf)
    return ctx


POLL_ERR = []


def poll(pg, expr, timeout=900):
    """Waits for a page expression by polling evaluate (wait_for_function evaluates a string, which the page's CSP sometimes refuses)."""
    t = time.time()
    while time.time() - t < timeout:
        try:
            if pg.evaluate(expr): return
        except Exception as e: POLL_ERR.append(str(e)[:120])
        _WAIT(0.05)
    raise TimeoutError(expr)


def wait_ready(pg):
    """qqReady and cross-origin isolated (a cold profile reloads once for it, browser.js isolateOnce)."""
    for _ in range(3000):
        try:
            if pg.evaluate("window.qqReady === true && crossOriginIsolated === true"): return
        except Exception: pass
        _WAIT(0.05)
    raise TimeoutError("page not ready and isolated")


def make_warm(p, site, tmpl):
    """A profile that has seen the site long enough for the service worker to finish its fill (the `.fill-done` mark in the app cache)."""
    if (tmpl / ".ok").exists(): return
    shutil.rmtree(tmpl, ignore_errors=True); site.hold = ()
    ctx = open_ctx(p, str(tmpl)); pg = ctx.pages[0] if ctx.pages else ctx.new_page(); pg.goto(f"http://127.0.0.1:{site.port}/"); wait_ready(pg)
    pg.evaluate("window.__fd = false; (async () => { for (;;) { for (const k of await caches.keys()) { if ((await (await caches.open(k)).keys()).some(r => r.url.endsWith('.fill-done'))) { window.__fd = true; return; } } await new Promise(r => setTimeout(r, 500)); } })()")
    poll(pg, "window.__fd === true", 600)
    nap(pg, 12000); ctx.close(); (tmpl / ".ok").write_text("1")


def gestures(pg):
    """20 Ctrl+wheel zooms (in, out), 20 Shift+drags and 5 clicks on the TIC; the page records input -> next frame and the draws."""
    pg.evaluate("__s.wrapDraw()")
    r = pg.evaluate("(() => { const b = document.querySelector('#dpanels .pnl canvas').getBoundingClientRect(); return [b.left, b.top, b.width, b.height]; })()")
    cx, cy = r[0] + r[2] * 0.5, r[1] + r[3] * 0.5; pg.mouse.move(cx, cy); busy0 = pg.evaluate("UPBUSY + OPENNOW.busy()"); ts = pg.evaluate("performance.now() - __s.t0")
    for i in range(20):
        pg.keyboard.down("Control"); pg.mouse.wheel(0, -100 if i % 2 == 0 else 100); pg.keyboard.up("Control"); nap(pg, 40)
    pg.keyboard.down("Shift")
    for i in range(20):
        pg.mouse.move(cx, cy); pg.mouse.down(); pg.mouse.move(cx + (60 if i % 2 else -60), cy, steps=3); pg.mouse.up(); nap(pg, 40)
    pg.keyboard.up("Shift"); clicks = []
    for i in range(5):
        pg.evaluate("window.__click = 0; __s.draws.length = 0"); pg.mouse.click(r[0] + r[2] * (0.2 + 0.15 * i), r[1] + r[3] * 0.4); nap(pg, 450)
        clicks.append(pg.evaluate("(() => { const d = __s.draws.filter(x => x[1] === 'spec'); return window.__click && d.length ? d[d.length - 1][0] - window.__click : null; })()"))
    return {"clicks": clicks, "busy_before": busy0, "busy_after": pg.evaluate("UPBUSY + OPENNOW.busy()"), "t_start": round(ts), "t_end": round(pg.evaluate("performance.now() - __s.t0"))}


def one_run(p, site, arm, cpu, files, tmpl):
    prof = tempfile.mkdtemp(prefix="mzlab-scen-"); shutil.rmtree(prof)
    if arm.get("warm"): shutil.copytree(tmpl, prof, symlinks=True); (Path(prof) / ".ok").unlink(missing_ok=True)
    site.hold, site.release = tuple(arm.get("hold", ())), None; site.log.clear()
    ctx = open_ctx(p, prof, copie=arm.get("copie")); pg = ctx.pages[0] if ctx.pages else ctx.new_page()
    sampler = RssSampler(prof) if arm.get("mem") else None
    try:
        pg.goto(f"http://127.0.0.1:{site.port}/"); wait_ready(pg)
        nap(pg, arm.get("wait", 3000)); cdp = ctx.new_cdp_session(pg)
        if cpu > 1: cdp.send("Emulation.setCPUThrottlingRate", {"rate": cpu})
        if sampler: sampler.start()
        if arm.get("mem"): pg.evaluate("__s.memOn = true; __s.memLoop()")
        site.release = time.time() * 1000 + DELAY_MS; pg.set_input_files("#pick", files[: arm["n"]]); t0e = pg.evaluate("__s.t0e")
        info = None
        if arm.get("act"): poll(pg, "!!__s.tchart"); info = gestures(pg)
        poll(pg, "!!__s.tall"); pg.evaluate("__s.memOn = false"); poll(pg, "__s.pend === 0", 40) if arm.get("mem") else None
        s = pg.evaluate("__s"); s["t0e"] = t0e; s["act"] = info
        s["srv"] = [[round(r[0] - t0e), r[1].split("/")[-1][:48], r[2], round(r[3] - r[0]) if r[3] else None] for r in site.log]
        if sampler: sampler.on = False; s["rss"] = sampler.v
        return s
    finally:
        try: ctx.close()
        except Exception: pass
        shutil.rmtree(prof, ignore_errors=True)


def pct(v, q):
    v = sorted(v); return round(v[min(len(v) - 1, int(len(v) * q))], 1) if v else None


def digest(s):
    """One run -> the numbers of the tables."""
    tc, ta = s.get("tchart"), s.get("tall"); lat = lambda k: [x[1] for x in s["lat"] if x[0].split(":")[0] == k]; busy = lambda k: [x[1] for x in s["lat"] if x[0] == k + ":busy"]
    first = [x[0] for x in s["idx"] if x[0] >= 0][:1]; fr = [x for x in s["fr"][1:] if x < 5000]
    et = [x[2] for x in s["et"] if tc and x[1] >= s["t0"] + tc]; srv = [r for r in s["srv"] if tc and 0 <= r[0] <= tc]
    o = {"tprov": s.get("tprov"), "tchart": tc, "tall": ta, "copy_start": first[0] if first else None,
         "lt_n": len([x for x in s["lt"] if tc and x[0] <= tc]), "lt_ms": sum(x[1] for x in s["lt"] if tc and x[0] <= tc),
         "srv_n": len(srv), "srv_mb": round(sum(r[2] for r in srv) / 1e6, 2), "srv": [r[1] for r in srv if r[2] > 100000][:8],
         "fr_p95": pct(fr, .95), "fr_max": round(max(fr), 1) if fr else None, "ev_p50": pct(et, .5), "ev_p98": pct(et, .98), "ev_n": len(et),
         "wheel_p50": pct(lat("wheel"), .5), "wheel_p95": pct(lat("wheel"), .95), "drag_p50": pct(lat("drag"), .5), "drag_p95": pct(lat("drag"), .95),
         "wheel_busy_n": len(busy("wheel")), "wheel_busy_p95": pct(busy("wheel"), .95), "drag_busy_n": len(busy("drag")), "drag_busy_p95": pct(busy("drag"), .95)}
    if s.get("act"): o["click_spec"] = pct([c for c in s["act"]["clicks"] if c is not None], .5); o["busy_in_gestures"] = s["act"]["busy_before"] > 0 and s["act"]["busy_after"] > 0
    if s.get("mem") or s.get("rss"): o.update(mem_peak_mb=round(max([m[1] for m in s["mem"]] or [0]) / 1e6), mem_peak_at=(max(s["mem"], key=lambda m: m[1])[3] if s["mem"] else None), mem_n=len(s["mem"]), heap_peak_mb=round(max([m[2] for m in s["mem"]] or [0]) / 1e6), rss_peak_mb=round(max(s.get("rss") or [0])), mem_err=s.get("memErr"))
    return o


def arms_of(name):
    cold, warm = {"warm": False}, {"warm": True}
    return {
        "uno": {"uno": dict(n=1, **warm), "dieci": dict(n=10, **warm), "raw-uno": dict(n=1, raw=1, **warm), "raw-dieci": dict(n=10, raw=1, **warm)},
        "rotte": {"base": dict(n=1, **cold), "rotte+3s": dict(n=1, hold=("rotte/mzlab_rotte_wasm_bg.wasm",), **cold)},
        "thermo": {"base": dict(n=1, raw=1, **cold), "thermo+3s": dict(n=1, raw=1, hold=("thermo/mzlab_lettori_wasm_bg.wasm",), **cold)},
        "raw-subito": {"raw+0": dict(n=1, raw=1, wait=0, **cold)},
        "subito": {"cold+0": dict(n=1, wait=0, **cold), "cold+20s": dict(n=1, wait=20000, **cold), "warm+0": dict(n=1, wait=0, **warm), "warm+20s": dict(n=1, wait=20000, **warm)},
        "interazione": {"dieci": dict(n=10, **warm), "dieci+gesti": dict(n=10, act=1, **warm), "uno+gesti": dict(n=1, act=1, **warm)},
        "memoria": {"dieci+mem": dict(n=10, mem=1, **warm)},
        # the copy gate (lr.js copiesOf, `copie` = copies at once, 0 = all at once as before the gate)
        "copie": {"uno": dict(n=1, **warm), **{f"dieci-k{k}": dict(n=10, copie=k, **warm) for k in (0, 1, 2, 3)}},
        "raw-copie": {"raw-uno": dict(n=1, raw=1, **warm), **{f"raw-dieci-k{k}": dict(n=10, raw=1, copie=k, **warm) for k in (0, 1, 2, 3)}},
        "copie-mem": {f"dieci+mem-k{k}": dict(n=10, mem=1, copie=k, **warm) for k in (0, 1, 2)},
        "cinquanta": {f"cinquanta-k{k}": dict(n=50, small=1, copie=k, **warm) for k in (0, 1, 2)},
    }[name]


def table(name, res):
    keys = ("tprov", "tchart", "tall", "copy_start", "lt_n", "lt_ms", "srv_n", "srv_mb", "fr_p95", "ev_p50", "ev_p98", "wheel_p95", "wheel_busy_n", "wheel_busy_p95", "drag_p95", "drag_busy_n", "drag_busy_p95", "click_spec", "mem_peak_mb", "rss_peak_mb")
    out = [f"\n### {name}\n\n| arm | CPU | runs | " + " | ".join(keys) + " |\n|" + "---|" * (len(keys) + 3)]
    for (arm, cpu), rs in res.items():
        cell = lambda k: (lambda v: "-" if not v else f"{statistics.median(v):g} [{min(v):g}-{max(v):g}]")([r[k] for r in rs if r.get(k) is not None])
        out.append(f"| {arm} | {cpu}x | {len(rs)} | " + " | ".join(cell(k) for k in keys) + " |")
    return "\n".join(out)


def main(a):
    wd = Path(a.wd); wd.mkdir(parents=True, exist_ok=True); site = Site(Path(a.site).resolve(), 8977); all_res = {}
    cpus = [int(x) for x in a.cpu.split(",")]; names = a.scenarios.split(",")
    try:
        with sync_playwright() as p:
            make_warm(p, site, wd / "warm-profile")
            for name in names:
                arms = arms_of(name); raw = {k: make_files(wd / ("raw" if v.get("raw") else "mzml50" if v.get("small") else "mzml"), bool(v.get("raw")), n=max(10, v["n"]), scans=480 if v.get("small") else None) for k, v in arms.items()}; res = {}
                for i in range(a.runs):
                    for cpu in cpus:
                        for arm, cfg in arms.items():
                            try: s = one_run(p, site, cfg, cpu, raw[arm], wd / "warm-profile")
                            except Exception as e: print(f"{name} {arm} {cpu}x run {i + 1}: FAILED {str(e)[:200]}", flush=True); continue
                            d = digest(s); d["raw_run"] = s; d["poll_err"] = len(POLL_ERR); POLL_ERR.clear(); res.setdefault((arm, cpu), []).append(d); print(f"{name} {arm} {cpu}x run {i + 1}: " + json.dumps({k: v for k, v in d.items() if k != "raw_run"}), flush=True)
                all_res[name] = res; print(table(name, res), flush=True)
    finally:
        site.close()
    if a.out: Path(a.out).write_text(json.dumps({n: {f"{k[0]}|{k[1]}": v for k, v in r.items()} for n, r in all_res.items()}))
