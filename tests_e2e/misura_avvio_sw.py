"""Probe of the boot path a returning user meets and misura_primo_paint.py does not: a service worker (SW) the browser stopped, a network with latency,
the V8 code cache, the order of the boot steps, and the isolation reload. Measures, never asserts (docs/agenti/misure-caricamento.md).

    python3 tests_e2e/misura_avvio_sw.py --mirror DIR [--runs 5] [--rtt 0,40,150] [--cpu 1,4] [--scenari warm,warm-restarted,cold,trace-warm,trace-restarted]
                                         [--out run.json]            # local mirror of --url with GitHub-Pages headers (filled from --url once)
    python3 tests_e2e/misura_avvio_sw.py --url https://freeesta.github.io/mzlab/ --rtt 0 --cpu 1 --scenari warm,warm-restarted,cold   # the live site
    python3 tests_e2e/misura_avvio_sw.py --varianti main=DIR0,A=DIR1,B=DIR1?nav=cache [--rtt ...]   # built sites side by side, one mirror each

Scenarios (every combination of --rtt and --cpu, interleaved over --runs, one new Chromium per run):
  cold            first visit, ends on the second page (after the isolation reload); also the bytes/requests of the first, non-isolated page.
  warm            first visit, wait for `.fill-done` in the app cache, then reload with the SW still running.
  warm-restarted  the same, but the SW is stopped over CDP (ServiceWorker.stopAllWorkers) before the reload.
  --warm-reloads N measures the Nth reload in warm / warm-restarted (default 1); every load also records its longest task before qqReady.
  trace-warm / trace-restarted (rtt 0 only)  third load under a Chromium trace (v8, v8.wasm, disabled-by-default-v8.compile, devtools.timeline).
--varianti NAME=DIR[?QUERY],...: every combination for every variant, interleaved; the first visit carries ?QUERY (a switch the SW remembers, such as nav=cache),
then one more load of the plain address before the measured reload, in every variant alike. All times are ms from the navigation start of the measured page. Boot steps come from the page itself (init script: script `load` events, the engine
worker's `ready` message, setters on window.qqReady / MZLAB_LR, a wrapper of window.fetch for api/state): no product file is touched.
"""
import argparse, json, os, re, statistics, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
from http.server import ThreadingHTTPServer  # noqa: E402
import threading  # noqa: E402
from misura_ricarica import Mirror, Throttle, serve_mirror  # noqa: E402
from misura_primo_paint import poll  # noqa: E402

SRC = (ROOT / "mzlab" / "web" / "carica.js").read_text(encoding="utf-8")
L = [n[2:] if n.startswith("m:") else n for n in re.findall(r'"([^"]+)"', re.search(r"var L = \[(.*?)\];", SRC, re.S).group(1))]
L_MOD = [n[2:] for n in re.findall(r'"([^"]+)"', re.search(r"var L = \[(.*?)\];", SRC, re.S).group(1)) if n.startswith("m:")]
SETTLE_MS = 6000
CATS = ["v8", "v8.wasm", "disabled-by-default-v8.compile", "devtools.timeline"]
FILL_DONE = "caches.keys().then(k => Promise.all(k.filter(n => n.startsWith('mzlab-app-')).map(n => caches.open(n).then(c => c.match('.fill-done'))))).then(a => a.some(Boolean))"

PROBE = r"""(() => {
  const T0 = performance.timeOrigin, now = () => Math.round(performance.now() * 10) / 10;
  const m = {}, scripts = [], v = window.__a = { m, scripts, t0: T0 };
  try { performance.setResourceTimingBufferSize(5000); } catch (_) {}
  try { new PerformanceObserver(l => l.getEntries().forEach(e => { if (e.name === "first-contentful-paint") m.fcp = Math.round(e.startTime * 10) / 10; })).observe({ type: "paint", buffered: true }); } catch (_) {}
  try { m.longest = 0; new PerformanceObserver(l => l.getEntries().forEach(e => { if (!m.ready && e.duration > (m.longest || 0)) m.longest = Math.round(e.duration); })).observe({ type: "longtask", buffered: true }); } catch (_) {}   // longest task before qqReady
  // a script's `load` fires after it ran (classic and module): the end of each script of the list L, and of shell.js
  document.addEventListener("load", e => { const t = e.target; if (t && t.tagName === "SCRIPT" && t.src) scripts.push([t.src.replace(location.origin, "").replace(/^\/[^/]*\/static\//, "").replace(/^\/static\//, ""), now()]); }, true);
  const prop = (name, f) => { let val; Object.defineProperty(window, name, { configurable: true, get: () => val, set: x => { val = x; f(x); } }); };
  prop("qqReady", x => { if (x === true && !m.ready) m.ready = now(); });
  prop("MZLAB_LR", x => { if (x && !m.engine) m.engine = now(); });          // set right before perfMark("motore pronto") in browser.js
  const W = window.Worker;
  window.Worker = class extends W { constructor(u, o) { super(u, o); const n = String(u), first = !m.workerNew; if (/rust-worker/.test(n) && first) m.workerNew = now();
    this.addEventListener("message", ev => { const d = ev.data; if (d && d.op === "ready" && /rust-worker/.test(n) && !m.workerReady) m.workerReady = now(); }); } };
  let wrapped;
  const wrap = f => function (i, o) { const u = typeof i === "string" ? i : i && i.url;
    if (typeof u === "string" && /^(\.\/)?api\/state/.test(u)) { if (!m.stateReq) m.stateReq = now(); return f.apply(this, arguments).then(r => { if (!m.stateEnd) m.stateEnd = now(); return r; }); }
    return f.apply(this, arguments); };
  wrapped = wrap(window.fetch.bind(window));
  Object.defineProperty(window, "fetch", { configurable: true, get: () => wrapped, set: f => { wrapped = wrap(f); } });
  const poll = () => { const d = document.getElementById("drop"); if (d && d.offsetParent && d.getBoundingClientRect().height > 0) { m.drop = now(); return; } setTimeout(poll, 8); };
  poll();
  v.snap = () => { const n = performance.getEntriesByType("navigation")[0] || {};
    v.nav = { type: n.type, workerStart: n.workerStart, fetchStart: n.fetchStart, requestStart: n.requestStart, responseStart: n.responseStart, responseEnd: n.responseEnd };
    v.res = performance.getEntriesByType("resource").map(e => [e.name.replace(location.origin, ""), Math.round(e.startTime), Math.round(e.responseEnd), e.transferSize, e.encodedBodySize]);
    v.coi = !!window.crossOriginIsolated; return JSON.parse(JSON.stringify(v)); };
  try { localStorage.setItem("mzlab.tour.lr", JSON.stringify({ stato: "saltato" })); localStorage.setItem("mzlab.barra.hint", "1"); localStorage.setItem("mzlab.pannelli.affianca.hint", "1"); } catch (_) {}
})();"""
USABLE = "!!(window.__a && __a.m.ready && __a.m.drop && window.crossOriginIsolated)"


def pump(pg, ms: int) -> None:
    """Settle time measured by the page itself (a timer in the page), not by the runner."""
    pg.evaluate(f"new Promise(r => setTimeout(r, {ms}))")


def wait_fill(pg, timeout: float = 300) -> None:
    """Until the SW's background fill has left `.fill-done`. FILL_DONE is a promise, so it is awaited here: poll() tests `!!(expr)`, true for any promise."""
    end = time.time() + timeout
    while not pg.evaluate(FILL_DONE):
        if time.time() > end:
            raise TimeoutError("fill not done")
        pump(pg, 200)


def wall() -> float:
    return time.time() * 1000


def med(xs):
    xs = [x for x in xs if x is not None]
    return (statistics.median(xs), min(xs), max(xs)) if xs else None


def fmt(t, nd=0):
    return "—" if t is None else (f"{t[0]:.{nd}f} [{t[1]:.{nd}f}-{t[2]:.{nd}f}]")


def stop_sw(ctx, pg) -> None:
    """Stops every service worker like the browser does after ~30 s idle; waits until CDP says it is stopped."""
    s = ctx.new_cdp_session(pg)
    st = {}
    s.on("ServiceWorker.workerVersionUpdated", lambda ev: st.update({v["versionId"]: v["runningStatus"] for v in ev["versions"]}))
    s.send("ServiceWorker.enable")
    pump(pg, 300)                                  # a page-side timer also delivers the CDP events (time.sleep would not)
    stopped = lambda: bool(st) and all(x == "stopped" for x in st.values())
    for _ in range(4):                              # a worker busy with a request (a last fill step) restarts: ask again
        s.send("ServiceWorker.stopAllWorkers")
        end = time.time() + 4
        while time.time() < end and not stopped():
            pump(pg, 100)
        if stopped():
            break
    if not stopped():
        raise RuntimeError(f"service worker not stopped: {st}")
    s.send("ServiceWorker.disable")


def page_metrics(snap: dict, thr: Throttle, mlog: list, sw0: int, l0: int) -> dict:
    """Times of the boot steps, ms from the navigation start of the measured page."""
    t0, m, nav = snap["t0"], snap["m"], snap["nav"]
    sc = {n: t for n, t in snap["scripts"]}
    classic = [t for n, t in snap["scripts"] if n in L and n not in L_MOD and n != "shell.js"]
    modules = [t for n, t in snap["scripts"] if n in L_MOD]
    out = {"idx_end": nav["responseEnd"], "sw_worker_start": nav["workerStart"], "fcp": m.get("fcp"), "last_classic": max(classic) if classic else None,
           "shell_js": sc.get("shell.js"), "last_module": max(modules) if modules else None, "worker_ready": m.get("workerReady"), "engine": m.get("engine"),
           "api_state_req": m.get("stateReq"), "api_state": m.get("stateEnd"), "qqReady": m.get("ready"), "drop": m.get("drop"), "longest_task": m.get("longest")}
    mf = [r for r in thr.sw[sw0:] if r[1].endswith("static/sw-manifest.json")]
    out["manifest_req"] = round(mf[0][0] - t0) if mf else None          # as the SW sends it (CDP of the SW target)
    out["manifest_end"] = round(mf[0][4] - t0) if mf else None
    sv = [r for r in mlog[l0:] if r[1].endswith("static/sw-manifest.json")]
    out["manifest_srv"] = round(sv[0][0] - t0) if sv else None          # arrival at the mirror (after the emulated latency)
    si = [r for r in mlog[l0:] if r[1] in ("", "index.html")]
    out["index_srv"] = round(si[0][0] - t0) if si else None             # the mirror's own arrival time of the page request
    # the last boot step: which of these ends the critical path to qqReady
    steps = {k: out[k] for k in ("last_classic", "shell_js", "last_module", "worker_ready", "engine", "api_state") if out.get(k) is not None}
    out["last_step"] = max(steps, key=steps.get) if steps else None
    return out


def serve_dir(folder: str) -> str:
    """A mirror of one built site (no upstream); the delay and the request log are the shared ones of `Mirror`."""
    m = type("M", (Mirror,), {"root": Path(folder), "upstream": "", "gz": {}})
    srv = ThreadingHTTPServer(("127.0.0.1", 0), m)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{srv.server_address[1]}/mzlab/"


def one(b, base, scen: str, rtt: int, cpu: int, mirror: bool, loads: int = 2, warm_loads: int = 1, query: str | None = None) -> dict:
    # latency: the mirror delays every answer (CDP latency does not apply to the SW's own fetches); the CDP profile only slows the CPU
    thr = Throttle(b, "cpu4" if cpu == 4 else "none")
    Mirror.delay = rtt if mirror else 0
    c = b.new_context(viewport={"width": 1400, "height": 900})
    c.add_init_script(PROBE)
    pg = c.new_page()
    url = base + ("?" + query if query else "") + "#/data"
    log = Mirror.log
    l_start = len(log)
    t_goto = wall()
    pg.goto(url)
    poll(pg, USABLE, 240)
    res = {}
    if scen == "cold":
        snap = pg.evaluate("__a.snap()")
        t2 = snap["t0"]
        e1 = [r for r in log[l_start:] if t_goto - 50 <= r[0] < t2 and not r[1].startswith("api/")]
        paths1 = {r[1] for r in e1}
        e2 = [r for r in log[l_start:] if t2 <= r[0] <= t2 + snap["m"]["ready"]]
        p2 = {r[0].split("?")[0].lstrip("/").replace("mzlab/", "", 1) for r in snap["res"] if r[2] <= snap["m"]["ready"]}
        net2 = {r[1]: r[2] for r in e2}
        both = {x for x in p2 if x in paths1 or x == ""}
        res.update(page_metrics(snap, thr, log, 0, l_start))
        res.update({"p1_req": len(e1), "p1_kb": round(sum(r[3] for r in e1) / 1024), "p2_files": len(p2), "p2_shared": len(both), "p2_net200": len([x for x in both if net2.get(x) == 200]), "p2_net304": len([x for x in both if net2.get(x) == 304]),
                    "p2_httpcache": len([x for x in both if x not in net2]), "p2_new": len(p2 - both), "p2_srv_req": len(e2),
                    "p2_srv_kb": round(sum(r[3] for r in e2) / 1024), "usable": snap["m"]["ready"]})
    else:
        wait_fill(pg)
        pump(pg, SETTLE_MS)           # the idle work after qqReady (prefetch at +3 s, the mzFinder package) ends before the measured load
        if query is not None:         # --varianti: the plain address once, so the measured reload carries no switch in its URL
            pg.goto(base + "#/data"); poll(pg, USABLE, 240); pump(pg, 1500)
        for _ in range((loads if scen.startswith("trace") else warm_loads) - 1):
            pg.reload(); poll(pg, USABLE, 240); pump(pg, 1500)        # loads 2..n-1; Chromium writes the code cache after the earlier ones
        if scen in ("warm-restarted", "trace-restarted"):
            stop_sw(c, pg)
        sw0, l0 = len(thr.sw), len(log)
        if scen.startswith("trace"):
            b.start_tracing(page=pg, categories=CATS)
        pg.reload()
        poll(pg, USABLE, 240)
        pump(pg, SETTLE_MS)           # the idle work after qqReady (prefetch at +3 s, the mzFinder package) ends before the measured load
        if scen.startswith("trace"):
            res["trace"] = analyse(json.loads(b.stop_tracing()))
            res["reloads"] = loads
        snap = pg.evaluate("__a.snap()")
        res.update(page_metrics(snap, thr, log, sw0, l0))
        res["sw_type"] = snap["nav"]["type"]
        res["reloads"] = loads if scen.startswith("trace") else warm_loads
    c.close()
    return res


def analyse(tr: dict) -> dict:
    """Compile and code-cache evidence from a Chromium trace. A hit is a `v8.compile`/`v8.compileModule` event whose args carry `consumedCacheSize` and not
    `cacheRejected` (found with a control page on a plain HTTP cache: the 3rd load shows it, the 1st does not); `v8.produceCache` is the cache being written."""
    ev = tr["traceEvents"] if isinstance(tr, dict) else tr
    tn = {(e["pid"], e["tid"]): e["args"]["name"] for e in ev if e.get("name") == "thread_name"}
    kind = lambda e: "main" if tn.get((e["pid"], e["tid"])) == "CrRendererMain" else ("worker" if "Worker" in tn.get((e["pid"], e["tid"]), "") and "Pool" not in tn.get((e["pid"], e["tid"]), "") else "other")
    eng = {(e["pid"], e["tid"]) for e in ev if e.get("name") in ("v8.compile", "v8.compileModule") and "rust-worker" in json.dumps(e.get("args"))}
    out: dict = {}
    for grp in ("L", "other", "worker"):
        out[grp] = {"scripts": 0, "hit": 0, "rejected": 0, "streamed": 0, "compile_ms": 0.0, "produce": 0}
    comp = [e for e in ev if e.get("name") in ("v8.compile", "v8.compileModule") and e.get("ph") == "X"]
    for e in comp:
        d = (e.get("args") or {}).get("data") or {}
        u = d.get("url", "")
        k = kind(e)
        if (e["pid"], e["tid"]) in eng or k == "worker":
            g = "worker"
        elif re.search(r"/static/([^/?#]+)$", u) and re.search(r"/static/([^/?#]+)$", u).group(1) in L:
            g = "L"
        else:
            g = "other"
        o = out[g]
        o["scripts"] += 1
        o["compile_ms"] += e.get("dur", 0) / 1000
        if "consumedCacheSize" in d and not d.get("cacheRejected"):
            o["hit"] += 1
        if d.get("cacheRejected"):
            o["rejected"] += 1
        if d.get("streamed"):
            o["streamed"] += 1
    out["produce_cache"] = len([e for e in ev if e.get("name") in ("v8.produceCache", "v8.produceModuleCache")])
    # function compiles that happen while a script runs (not inside the compile call above) and the background threads
    spans: dict = {}
    for e in comp:
        spans.setdefault((e["pid"], e["tid"]), []).append((e["ts"], e["ts"] + e.get("dur", 0)))
    lazy = {"main": 0.0, "worker": 0.0}
    for e in ev:
        if e.get("name") == "V8.CompileCode" and e.get("ph") == "X" and kind(e) in lazy:
            if not any(a <= e["ts"] <= z for a, z in spans.get((e["pid"], e["tid"]), [])):
                lazy[kind(e)] += e.get("dur", 0) / 1000
    out["lazy_ms"] = lazy
    out["background_ms"] = sum(e.get("dur", 0) for e in ev if e.get("name") == "BackgroundCompileTask::Run") / 1000
    w = [e for e in ev if str(e.get("name", "")).startswith("wasm.") and e.get("ph") == "X"]
    out["wasm"] = {"streaming_started": len([e for e in w if e["name"] == "wasm.StartStreamingCompilation"]),
                   "finish_streaming_ms": sum(e["dur"] for e in w if e["name"] == "wasm.FinishStreaming") / 1000,
                   "bytes_received_ms": sum(e["dur"] for e in w if e["name"] == "wasm.OnBytesReceived") / 1000,
                   "native_module_cache_ms": sum(e["dur"] for e in w if e["name"] == "wasm.GetNativeModuleFromCache") / 1000,
                   "worker_ms": sum(e["dur"] for e in w if e["name"] in ("wasm.SyncInstantiate", "wasm.AsyncInstantiate") and kind(e) == "worker") / 1000,
                   "names": sorted({e["name"] for e in w})}
    out["events"] = {n: len([e for e in ev if e.get("name") == n]) for n in ("v8.compile", "v8.compileModule", "V8.CompileScript", "CacheHit", "V8.StreamingFinalization.CheckCache", "v8.deserializeOnBackground")}
    return out


KEYS = ["idx_end", "index_srv", "sw_worker_start", "manifest_req", "manifest_end", "manifest_srv", "fcp", "last_classic", "shell_js", "last_module", "worker_ready", "engine", "api_state", "qqReady"]


def report(results: dict) -> None:
    """Markdown tables of the medians [min-max]."""
    def rows(pick, title, cols):
        print(f"\n### {title}\n\n| variant | scenario | RTT ms | CPU | runs | " + " | ".join(cols) + " |\n|---|---|---|---|---|" + "---|" * len(cols))
        for (var, scen, rtt, cpu), runs in sorted(results.items(), key=lambda kv: (kv[0][1], kv[0][3], kv[0][2], kv[0][0])):
            if not pick(scen) or not runs:
                continue
            print(f"| {var or '—'} | {scen} | {rtt} | ×{cpu} | {len(runs)} | " + " | ".join(fmt(med([r.get(k) for r in runs])) for k in cols) + " |")
    rows(lambda s: s in ("warm", "warm-restarted"), "Warm and warm-restarted: navigation start → …", KEYS[:7] + ["worker_ready", "engine", "qqReady", "longest_task"])
    rows(lambda s: s in ("cold", "warm", "cpu4warm", "warm-restarted"), "Boot order", ["last_classic", "shell_js", "last_module", "worker_ready", "api_state", "engine", "qqReady"])
    for (var, scen, rtt, cpu), runs in results.items():
        if runs and scen in ("cold", "warm", "warm-restarted"):
            c = {}
            for r in runs:
                c[r.get("last_step")] = c.get(r.get("last_step"), 0) + 1
            print(f"last step before qqReady, {var or '—'} {scen} rtt {rtt} ×{cpu}: {c}")
    rows(lambda s: s == "cold", "Isolation reload (cold)", ["p1_req", "p1_kb", "p2_files", "p2_shared", "p2_net200", "p2_net304", "p2_httpcache", "p2_new", "usable"])
    for (var, scen, rtt, cpu), runs in results.items():
        if scen.startswith("trace") and runs:
            print(f"\n### Code cache, {var or '—'} {scen} ×{cpu}, traced load = reload {runs[0]['reloads']} after the first visit ({len(runs)} runs)\n\n| group | compile events | hits | rejected | streamed | compile ms (main/worker thread, eager) |\n|---|---|---|---|---|---|")
            for g in ("L", "other", "worker"):
                print(f"| {g} | " + " | ".join(fmt(med([r['trace'][g][k] for r in runs])) for k in ("scripts", "hit", "rejected", "streamed")) + f" | {fmt(med([r['trace'][g]['compile_ms'] for r in runs]), 1)} |")
            print("produce_cache events", fmt(med([r["trace"]["produce_cache"] for r in runs])), "| lazy ms main", fmt(med([r["trace"]["lazy_ms"]["main"] for r in runs]), 1), "worker", fmt(med([r["trace"]["lazy_ms"]["worker"] for r in runs]), 1),
                  "| background-thread ms", fmt(med([r["trace"]["background_ms"] for r in runs]), 1))
            print("wasm:", {k: fmt(med([r["trace"]["wasm"][k] for r in runs]), 1) for k in ("streaming_started", "finish_streaming_ms", "bytes_received_ms", "native_module_cache_ms", "worker_ms")})


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="https://freeesta.github.io/mzlab/")
    ap.add_argument("--mirror", help="serve a local mirror from this folder (filled from --url on demand); without it the live --url is measured")
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--rtt", default="0,40,150", help="ms every answer of the mirror waits (ignored for the live site)")
    ap.add_argument("--cpu", default="1,4")
    ap.add_argument("--scenari", default="warm,warm-restarted,cold,trace-warm,trace-restarted")
    ap.add_argument("--trace-reloads", type=int, default=2, help="reloads after the first visit in the trace scenarios; the last one is traced (the code cache may need more than 2)")
    ap.add_argument("--warm-reloads", type=int, default=1, help="reloads after the first visit in warm / warm-restarted; the last one is measured (3 = the 4th load)")
    ap.add_argument("--varianti", help="NAME=DIR[?QUERY],...: built sites measured side by side (one mirror each, no --url); ?QUERY goes on the first visit")
    ap.add_argument("--out")
    a = ap.parse_args()
    from playwright.sync_api import sync_playwright
    variants: dict = {}       # name -> (base, query or None)
    if a.varianti:
        served: dict = {}
        for item in a.varianti.split(","):
            name, spec = item.split("=", 1)
            d, _, q = spec.partition("?")
            served.setdefault(d, serve_dir(d))
            variants[name] = (served[d], q)
    else:
        variants[""] = (serve_mirror(a.mirror, a.url) if a.mirror else a.url, None)
    combos = []
    for s in a.scenari.split(","):
        for cpu in [int(x) for x in a.cpu.split(",")]:
            for rtt in ([0] if s.startswith("trace") else [int(x) for x in a.rtt.split(",")]):
                for v in variants:
                    combos.append((v, s, rtt, cpu))
    results: dict = {k: [] for k in combos}
    exe = "/opt/pw-browsers/chromium"
    proxy = {"server": os.environ["HTTPS_PROXY"]} if os.environ.get("HTTPS_PROXY") and not (a.mirror or a.varianti) else None
    with sync_playwright() as p:
        for i in range(a.runs):                 # interleaved: every combination once per round
            for k in combos:
                b = p.chromium.launch(proxy=proxy, **({"executable_path": exe} if os.path.isfile(exe) else {}))
                try:
                    base, q = variants[k[0]]
                    results[k].append(one(b, base, *k[1:], mirror=bool(a.mirror or a.varianti), loads=a.trace_reloads, warm_loads=a.warm_reloads, query=q))
                    print(f"run {i + 1} {k}: ok", flush=True)
                except Exception as e:
                    print(f"run {i + 1} {k}: {str(e)[:200]}", flush=True)
                b.close()
                if a.out:
                    Path(a.out).write_text(json.dumps({"|".join(map(str, k)): v for k, v in results.items()}))
    report(results)


if __name__ == "__main__":
    main()
