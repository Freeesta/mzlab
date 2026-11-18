"""First-package probe of the built site (what the browser must download and run before the page is usable), for docs/agenti/measurements-loading.md.
Measures, never asserts; not run in CI.

Local mode (default): serves `site/` over HTTP/1.1 with GitHub-Pages-like headers and gzip -9 (GitHub Pages sends gzip, never brotli: checked with
`curl -H 'Accept-Encoding: br, gzip'` on the live site, answer `content-encoding: gzip`); the link is shaped by the server (tools/slow_network.py: one shared
downlink + a delay before each answer), because CDP throttling does not reach localhost or the service worker. Files missing from `site/` (the engine
.wasm files are built by the workflow, not by tools/build_site.py) are fetched once from --upstream during `--prime`.
Compare mode (`--compare URL ...`): real network through the proxy of the runner, CDP throttling of the page, same marks (FCP, requests, bytes).

    python3 tools/build_site.py && python3 tests_e2e/misura_payload.py --site site --prime
    python3 tests_e2e/misura_payload.py --site site --profiles none,fast4g,slow4g --cpus 1,4 --modes cold,warm --runs 3 --out DIR
    python3 tests_e2e/misura_payload.py --compare https://htmx.org/ https://www.sqlite.org/ --profiles none,fast4g --out DIR

Per run, one JSON with the request table (every file of the visit, request order: url, kind, encoded and decoded bytes, start, end, blocking/preload/
inserted/engine flags), the milestones (FCP, drop area usable, last carica.js script, engine ready) and the cumulative bytes/requests at each, and (cold, --trace)
the main-thread time by category from a Chrome trace. Times are ms from the start of the first navigation of the visit.
"""
from __future__ import annotations

import argparse, email.utils, gzip, json, mimetypes, os, re, statistics, sys, threading, time, urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e")); sys.path.insert(0, str(ROOT / "tools"))
import misura_ricarica as mr  # noqa: E402  (PROBE, Throttle)
import slow_network  # noqa: E402
import misura_payload_rapporto as rap  # noqa: E402

# name: (downlink bit/s, delay before each answer in s); the DevTools «Fast 4G» is 9 Mbit/s with 170 ms RTT, «Slow 4G» 1.6 Mbit/s with 150 ms
PROFILES = {"none": None, "fast4g": (9_000_000, 0.170), "slow4g": (1_600_000, 0.150)}
GZ_TYPES = ("text/", "application/javascript", "application/json", "application/wasm", "image/svg+xml")


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    root = Path("."); upstream = ""; link = None; rtt = 0.0; log: list = []; gz: dict = {}; lock = threading.Lock()

    def log_message(self, *a):
        pass

    def setup(self):
        super().setup()
        if self.link:
            self.wfile = slow_network._SlowFile(self.wfile, self.link, 4096)

    def _file(self, rel):
        p = self.root / rel if rel and not rel.endswith("/") else self.root / rel / "index.html"
        if p.is_file():
            return p
        if not self.upstream or p.with_name(p.name + ".404").exists():
            return None
        try:
            with urllib.request.urlopen(self.upstream + rel, timeout=120) as r:
                data = r.read()
            p.parent.mkdir(parents=True, exist_ok=True); p.write_bytes(data)
            return p
        except Exception:
            p.parent.mkdir(parents=True, exist_ok=True); p.with_name(p.name + ".404").write_bytes(b"")
            return None

    def do_GET(self):
        t0 = time.time() * 1000
        rel = self.path.split("?", 1)[0].split("#", 1)[0].lstrip("/")
        rel = rel[len("mzlab/"):] if rel.startswith("mzlab/") else rel
        p = self._file(rel)
        if self.rtt:
            threading.Event().wait(self.rtt)   # the modelled answer delay of the link (server side), not a wait of the page
        if p is None:
            self.send_response(404); self.send_header("Content-Length", "0"); self.end_headers(); return
        st = p.stat(); etag = 'W/"%x-%x"' % (int(st.st_mtime), st.st_size)
        if self.headers.get("If-None-Match") == etag:
            self.send_response(304); self.send_header("ETag", etag); self.send_header("Cache-Control", "max-age=600"); self.end_headers()
            self.log.append({"url": "/" + rel, "t0": t0, "t1": time.time() * 1000, "status": 304, "wire": 0, "dec": 0}); return
        ctype = mimetypes.guess_type(p.name)[0] or "application/octet-stream"
        ctype = "application/javascript" if p.suffix in (".js", ".mjs") else "application/wasm" if p.suffix == ".wasm" else ctype
        data = p.read_bytes(); dec = len(data); enc = None
        if "gzip" in self.headers.get("Accept-Encoding", "") and ctype.startswith(GZ_TYPES):
            with self.lock:
                key = (str(p), etag)
                if key not in self.gz:
                    self.gz[key] = gzip.compress(data, 9)
                data = self.gz[key]
            enc = "gzip"
        self.send_response(200)
        self.send_header("Content-Type", ctype + ("; charset=utf-8" if ctype.startswith("text/") or ctype == "application/javascript" else ""))
        self.send_header("Content-Length", str(len(data))); self.send_header("ETag", etag)
        self.send_header("Last-Modified", email.utils.formatdate(st.st_mtime, usegmt=True)); self.send_header("Cache-Control", "max-age=600")
        self.send_header("Access-Control-Allow-Origin", "*")
        if enc:
            self.send_header("Content-Encoding", enc); self.send_header("Vary", "Accept-Encoding")
        self.end_headers(); self.wfile.write(data)
        self.log.append({"url": "/" + rel, "t0": t0, "t1": time.time() * 1000, "status": 200, "wire": len(data), "dec": dec, "enc": enc or "identity"})


def serve(site: str, profile: str, upstream: str = ""):
    cls = type("H", (Handler,), {"root": Path(site), "upstream": upstream, "log": [], "gz": Handler.gz, "lock": Handler.lock})
    if PROFILES[profile]:
        rate, rtt = PROFILES[profile]
        cls.link = slow_network.Link(rate / 8); cls.rtt = rtt
    srv = ThreadingHTTPServer(("127.0.0.1", 0), cls)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, cls, f"http://127.0.0.1:{srv.server_address[1]}/mzlab/"


NODE_H2 = r"""
const http2 = require('http2'), fs = require('fs'), zlib = require('zlib'), path = require('path');
const [root, port, rate, rtt, keyf, certf] = process.argv.slice(2);
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'application/javascript; charset=utf-8', '.mjs': 'application/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8', '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.png': 'image/png', '.ico': 'image/x-icon', '.csv': 'text/csv; charset=utf-8', '.webmanifest': 'application/manifest+json', '.txt': 'text/plain; charset=utf-8', '.woff2': 'font/woff2' };
const GZ = /^(text\/|application\/(javascript|json|wasm)|image\/svg)/;
const cache = new Map(); let log = []; let free = 0;
const take = n => { const now = Date.now(); free = Math.max(now, free) + (rate > 0 ? n / rate * 1000 : 0); return free - now; };
const sleep = ms => new Promise(r => setTimeout(r, ms));
http2.createSecureServer({ key: fs.readFileSync(keyf), cert: fs.readFileSync(certf) }, async (req, res) => {
  const t0 = Date.now();
  let rel = decodeURIComponent(req.url.split('?')[0].split('#')[0]).replace(/^\/+/, '');
  if (rel === '__log') { res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(log)); log = []; return; }
  if (rel.startsWith('mzlab/')) rel = rel.slice(6);
  let p = path.join(root, rel === '' || rel.endsWith('/') ? rel + 'index.html' : rel);
  if (!fs.existsSync(p) || fs.statSync(p).isDirectory()) { await sleep(+rtt); res.writeHead(404, { 'content-length': 0 }); res.end(); return; }
  const st = fs.statSync(p), etag = 'W/"' + st.mtimeMs.toString(16) + '-' + st.size + '"';
  await sleep(+rtt);
  if (req.headers['if-none-match'] === etag) { res.writeHead(304, { etag, 'cache-control': 'max-age=600' }); res.end(); log.push({ url: '/' + rel, t0, t1: Date.now(), status: 304, wire: 0, dec: 0 }); return; }
  const ct = TYPES[path.extname(p)] || 'application/octet-stream';
  let data = fs.readFileSync(p); const dec = data.length; let enc = 'identity';
  const ae = req.headers['accept-encoding'] || '';
  if (ae.includes('gzip') && GZ.test(ct)) { const k = p + etag; if (!cache.has(k)) cache.set(k, zlib.gzipSync(data, { level: 9 })); data = cache.get(k); enc = 'gzip'; }
  const h = { 'content-type': ct, 'content-length': data.length, etag, 'last-modified': new Date(st.mtimeMs).toUTCString(), 'cache-control': 'max-age=600', 'access-control-allow-origin': '*' };
  if (enc === 'gzip') { h['content-encoding'] = 'gzip'; h['vary'] = 'Accept-Encoding'; }
  res.writeHead(200, h);
  for (let i = 0; i < data.length; i += 16384) {
    const c = data.subarray(i, i + 16384); const w = take(c.length); if (w > 1) await sleep(w);
    if (res.destroyed) return; res.write(c);
  }
  res.end(); log.push({ url: '/' + rel, t0, t1: Date.now(), status: 200, wire: data.length, dec, enc });
}).listen(+port, '127.0.0.1', () => console.log('listening'));
"""


def serve_h2(site: str, profile: str):
    """Start the Node h2 server; returns (stop, getlog, base). One self-signed certificate per process (Chromium runs with --ignore-certificate-errors)."""
    import socket, subprocess, tempfile
    d = Path(tempfile.mkdtemp(prefix="mzlab-h2-"))
    key, cert = d / "k.pem", d / "c.pem"
    subprocess.run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", str(key), "-out", str(cert), "-days", "2", "-subj", "/CN=127.0.0.1", "-addext", "subjectAltName=IP:127.0.0.1"], check=True, capture_output=True)
    (d / "srv.js").write_text(NODE_H2)
    with socket.socket() as so:
        so.bind(("127.0.0.1", 0)); port = so.getsockname()[1]
    rate, rtt = PROFILES[profile] or (0, 0)
    proc = subprocess.Popen(["node", str(d / "srv.js"), str(Path(site).resolve()), str(port), str(rate / 8), str(rtt * 1000), str(key), str(cert)], stdout=subprocess.PIPE, text=True)
    proc.stdout.readline()
    base = f"https://127.0.0.1:{port}/mzlab/"

    def getlog():
        # curl speaks h2 (urllib would not) and ignores the runner proxy for the loopback
        out = subprocess.run(["curl", "-sk", "--noproxy", "*", f"https://127.0.0.1:{port}/__log"], capture_output=True, text=True, check=True).stdout
        return json.loads(out)
    return (lambda: proc.kill()), getlog, base



def poll(pg, expr, timeout=240.0):
    js = f"new Promise(ok => {{ const end = Date.now() + 1000, f = () => {{ let v = false; try {{ v = !!({expr}); }} catch (_) {{}} if (v || Date.now() > end) ok(v); else setTimeout(f, 20); }}; f(); }})"
    end = time.time() + timeout
    while time.time() < end:
        try:
            if pg.evaluate(js):
                return True
        except Exception:
            pass
    raise TimeoutError(expr)


def static_sets(site: str):
    """What index.html and carica.js declare: render-blocking scripts/styles, preloads, the carica list in its load order."""
    html = (Path(site) / "index.html").read_text(encoding="utf-8")
    head = html.split("</head>")[0]
    scripts = set(re.findall(r'<script src="(static/[^"]+)"', html))
    styles = set(re.findall(r'<link rel="stylesheet" href="(static/[^"]+)"', html))
    pre = set(re.findall(r'<link rel="preload"[^>]*href="(static/[^"]+)"', html))
    c = (Path(site) / "static" / "carica.js").read_text(encoding="utf-8")
    L = re.search(r'var L = \[(.*?)\];', c, re.S).group(1)
    lst = [x.split(":")[-1] for x in re.findall(r'"([^"]+)"', L)]
    classic = [x for x in re.findall(r'"([^"]+)"', L) if not x.startswith("m:")]
    mods = [x[2:] for x in re.findall(r'"([^"]+)"', L) if x.startswith("m:")]
    return {"scripts": scripts, "styles": styles, "preload": pre, "carica": [x.split(":")[-1] for x in classic] + ["shell.js"] + mods}


_SZ: dict = {}


def disk_size(site: str, url: str):
    """(gzip -9 bytes, decoded bytes) of a served file: what a static host sends. The page API cannot tell it for files answered by the service worker cache."""
    rel = url.split("?", 1)[0].lstrip("/"); rel = rel[len("mzlab/"):] if rel.startswith("mzlab/") else rel
    p = Path(site) / (rel if rel and not rel.endswith("/") else rel + "index.html")
    if p not in _SZ:
        if not p.is_file():
            _SZ[p] = None
        else:
            d = p.read_bytes(); ct = mimetypes.guess_type(p.name)[0] or ""
            gz = p.suffix in (".js", ".mjs", ".wasm", ".json", ".css", ".html", ".svg", ".csv", ".txt", ".webmanifest") or ct.startswith("text/")
            _SZ[p] = (len(gzip.compress(d, 9)) if gz else len(d), len(d))
    return _SZ[p]


def kind_of(url: str, ent, sets) -> str:
    rel = url.lstrip("/"); rel = rel[len("mzlab/"):] if rel.startswith("mzlab/") else rel
    name = rel[len("static/"):] if rel.startswith("static/") else rel
    if url.endswith(".wasm"):
        return "engine (.wasm)"
    if rel in sets["styles"]:
        return "css (render-blocking)"
    if rel in sets["scripts"] and ent and ent.get("block") == "blocking":
        return "script (parser, render-blocking)"
    if name in sets["carica"]:
        return "script inserted by carica.js"
    if rel in sets["scripts"]:
        return "script (parser)"
    if rel in sets["preload"]:
        return "preload"
    if url.endswith((".png", ".ico", ".svg", ".webp")):
        return "image"
    if url.endswith((".html", "/")):
        return "document"
    if url.endswith(".js"):
        return "script (other, ES module / worker / catalog)"
    return ent.get("init", "other") if ent else "other"


def run_visit(b, base, mode, cpu, trace_path=None):
    """One visit. cold: new context, first visit (service worker installs, the page reloads once for isolation); warm: second visit in the same context."""
    thr = mr.Throttle(b, "cpu4") if cpu > 1 else None
    c = b.new_context(viewport={"width": 1400, "height": 900})
    c.add_init_script(mr.PROBE)
    pg = c.new_page()
    usable = "!!(window.__v && __v.marks.ready && __v.marks.drop && window.crossOriginIsolated)"
    if trace_path:
        b.start_tracing(page=pg, path=trace_path, categories=["devtools.timeline", "v8.execute", "v8", "disabled-by-default-devtools.timeline", "loading", "blink.user_timing", "toplevel"])
    pg.goto(base + "#/data"); poll(pg, usable)
    snaps = []
    if mode == "warm":
        # first visit over; wait until its service worker has filled the cache in the background, then measure the second one
        poll(pg, "caches.keys().then(k => Promise.all(k.filter(n => n.startsWith('mzlab-app-')).map(n => caches.open(n).then(c => c.match('.fill-done'))))).then(a => a.some(Boolean))", 600)
        pg.evaluate("localStorage.removeItem('__probe')")
        pg.reload(); poll(pg, usable)
    # let the boot tail finish: no long task for 1.5 s
    poll(pg, "(() => { const l = __v.lt[__v.lt.length - 1]; return !l || Date.now() - (l[0] + l[1]) > 1500; })()", 30)
    cur = pg.evaluate("JSON.parse(JSON.stringify(__v.snap()))")
    prev = pg.evaluate("JSON.parse(localStorage.getItem('__probe') || '[]')")
    if trace_path:
        b.stop_tracing()
    c.close()
    # warm: only the measured page (the snapshot of the page that was reloaded is the first visit's)
    return {"pages": ([cur] if mode == "warm" else prev + [cur]), "cpu": cpu}


def analyse(visit, srvlog, sets, site):
    """From the page snapshots (every navigation of the visit) and the server log: table + milestones, ms from the start of the first navigation."""
    pages = visit["pages"]
    t0 = pages[0]["t0"]; last = pages[-1]
    # a file counts once: the last page that really got it from the network (transferSize > 0), else the last entry of the final page
    rows = {}
    for pi, pg in enumerate(pages):
        for r in pg["res"]:
            url, init, st, dur, tr, enc, dec, block = r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]
            key = url
            ent = {"url": url, "init": init, "start": st - t0, "end": st + dur - t0, "transfer": tr, "enc": enc, "dec": dec, "block": block, "page": pi,
                   "net": tr > 0, "proto": r[8]}
            if key not in rows or (ent["net"] and not rows[key]["net"]):
                rows[key] = ent
    # the document of every page
    for pi, pg in enumerate(pages):
        n = pg["nav"]
        rows[f"@doc{pi}"] = {"url": "/mzlab/ (document)" + ("" if pi == len(pages) - 1 else " [first navigation]"), "init": "navigation", "start": n["reqStart"] - t0, "end": n["dcl"] - t0 if False else n["ttfb"] - t0,
                             "transfer": n.get("transfer", 0), "enc": n.get("transfer", 0), "dec": 0, "block": "blocking", "page": pi, "net": True, "proto": n.get("proto", "")}
    for e in rows.values():
        z = disk_size(site, e["url"].replace(" (document)", "").split(" [")[0])
        if z:
            e["enc"], e["dec"] = z
    table = []
    for e in sorted(rows.values(), key=lambda e: (e["start"], e["url"])):
        e["kind"] = kind_of(e["url"], e, sets)
        e["preload_flag"] = e["url"].lstrip("/").split("mzlab/")[-1] in sets["preload"]
        table.append(e)
    # decoded size of the documents from the server log (the page API reports 0 for navigation decoded size here)
    marks = {}
    for pi, pg in enumerate(pages):
        for k, v in pg["marks"].items():
            marks.setdefault(k, []).append(v - t0)
    fcp = marks.get("fcp", [None])[0]
    drop = marks.get("drop", [None])[0]
    ready = marks["ready"][0] if marks.get("ready") else None
    lastc = [e["end"] for e in table if e["url"].endswith(tuple("/" + n for n in sets["carica"]))]
    ms = {"fcp": fcp, "lcp": (marks.get("lcp") or [None])[-1], "drop": drop, "carica_last": max(lastc) if lastc else None, "ready": ready}
    cum = {}
    for name, t in ms.items():
        if t is None:
            continue
        done = [e for e in table if e["end"] <= t and not e["url"].endswith("(document) [first navigation]")]
        cum[name] = {"t": round(t), "req": len(done), "enc_kb": round(sum(e["enc"] for e in done) / 1024, 1), "dec_kb": round(sum(e["dec"] for e in done) / 1024, 1)}
    # the critical chain: everything finished until engine ready (or the last carica script, whichever is later)
    stop = max(x for x in (ms["ready"], ms["carica_last"]) if x) if (ms["ready"] or ms["carica_last"]) else None
    chain = [e for e in table if stop and e["end"] <= stop and not e["url"].endswith("[first navigation]")]
    wire = [r for r in srvlog if r["status"] == 200]
    return {"ms": {k: (round(v) if v is not None else None) for k, v in ms.items()}, "cum": cum, "table": table,
            "chain": {"req": len(chain), "enc_kb": round(sum(e["enc"] for e in chain) / 1024, 1), "dec_kb": round(sum(e["dec"] for e in chain) / 1024, 1)},
            "server": {"req": len(wire), "wire_kb": round(sum(r["wire"] for r in wire) / 1024, 1), "dec_kb": round(sum(r["dec"] for r in wire) / 1024, 1),
                       "n304": len([r for r in srvlog if r["status"] == 304])},
            "lt": [[round(x[0] - t0), x[1]] for x in last["lt"]], "coi": True,
            "nav": [{"ttfb": round(p["nav"]["ttfb"] - t0), "dcl": round(p["nav"]["dcl"] - t0), "load": round(p["nav"]["load"] - t0), "type": p["nav"].get("type")} for p in pages]}


# --- trace: main-thread time by category up to engine ready ------------------------------------------------------------------------------------------
CAT = {"script": ("EvaluateScript", "v8.run", "FunctionCall", "v8.callFunction", "TimerFire", "EventDispatch", "FireIdleCallback", "RunMicrotasks", "FireAnimationFrame", "XHRReadyStateChange", "v8.compile", "V8.Compile", "CompileScript", "v8.parseOnBackground", "V8.CompileCode", "CacheScript"),
       "parse": ("ParseHTML", "ParseAuthorStyleSheet", "ResourceFinish", "XHRLoad"),
       "render": ("UpdateLayoutTree", "Layout", "PrePaint", "Paint", "Layerize", "Commit", "UpdateLayer", "HitTest", "RasterTask", "ImageDecodeTask", "CompositeLayers", "IntersectionObserverController::computeIntersections", "RecalculateStyles", "ScheduleStyleRecalculation", "PaintImage"),
       "wasm": ("v8.wasm.compileLazy", "v8.wasm.streamFromResponseCallback", "v8.wasm.moduleCompile", "wasm.CompileLazy", "v8.wasm.compiledModule")}


def trace_breakdown(path, t_ready_ms, t_fcp_ms, nav1_epoch_ms, pages_epoch):
    data = json.loads(Path(path).read_text())
    ev = data["traceEvents"] if isinstance(data, dict) else data
    names = {}
    for e in ev:
        if e.get("ph") == "M" and e.get("name") == "thread_name":
            names[(e["pid"], e["tid"])] = e["args"]["name"]
    main = [k for k, v in names.items() if v == "CrRendererMain"]
    cnt = {}
    for e in ev:
        k = (e.get("pid"), e.get("tid"))
        if k in main and e.get("ph") == "X":
            cnt[k] = cnt.get(k, 0) + 1
    # every renderer main thread (the isolation reload may move the page to another process); events of one thread are processed on their own
    nav = [e for e in ev if e.get("name") == "navigationStart"]
    if not nav or not main:
        return None
    start = min(e["ts"] for e in nav)
    out = {}
    for label, limit in (("fcp", t_fcp_ms), ("ready", t_ready_ms)):
        if not limit:
            continue
        end = start + limit * 1000
        acc = {"script": 0.0, "parse": 0.0, "render": 0.0, "wasm": 0.0, "other_busy": 0.0}
        top = []
        for key in main:
            evs = sorted([e for e in ev if (e.get("pid"), e.get("tid")) == key and e.get("ph") == "X" and e.get("dur", 0) > 0], key=lambda e: (e["ts"], -e["dur"]))
            stack = []
            for e in evs:
                if e["ts"] >= end or e["ts"] + e["dur"] <= start:
                    continue
                s_, f_ = max(e["ts"], start), min(e["ts"] + e["dur"], end)
                while stack and stack[-1][1] <= e["ts"]:
                    stack.pop()
                cat = next((c for c, names_ in CAT.items() if e["name"] in names_), None)
                if stack:
                    acc[stack[-1][2]] -= (f_ - s_) / 1000   # the parent was counted whole: take the child's time off it
                else:
                    top.append(((s_ - start) / 1000, (f_ - start) / 1000))
                kind = cat or ("other_busy" if not stack else stack[-1][2])
                acc[kind] += (f_ - s_) / 1000
                stack.append((e["ts"], e["ts"] + e["dur"], kind))
        busy = sum(max(v, 0) for v in acc.values())
        out[label] = {"window_ms": limit, **{k: round(max(v, 0)) for k, v in acc.items()}, "busy_ms": round(busy), "idle_ms": round(limit - busy)}
        if label == "ready":
            out["_top"] = top
    return out


def merge(iv):
    out = []
    for s_, f_ in sorted(iv):
        if out and s_ <= out[-1][1]:
            out[-1][1] = max(out[-1][1], f_)
        else:
            out.append([s_, f_])
    return out


def span(iv):
    return sum(f_ - s_ for s_, f_ in iv)


def idle_split(table, top, stop):
    """Within [0, stop]: main thread idle while some request of the page is in flight (network wait) vs idle with nothing in flight (other)."""
    busy = merge([(max(a, 0), min(b, stop)) for a, b in top if b > 0 and a < stop])
    fl = merge([(max(e["start"], 0), min(e["end"], stop)) for e in table if e["end"] > 0 and e["start"] < stop and not e["url"].endswith("[first navigation]") or e["url"].endswith("[first navigation]") and e["start"] < stop])
    union = span(merge(busy + fl))
    return {"busy_ms": round(span(busy)), "inflight_ms": round(span(fl)), "idle_waiting_net_ms": round(span(merge(busy + fl)) - span(busy)), "idle_nothing_ms": round(stop - union)}


def launch(p):
    exe = "/opt/pw-browsers/chromium"
    kw = {"executable_path": exe} if Path(exe).is_file() else {}
    if os.environ.get("HTTPS_PROXY"):
        kw["proxy"] = {"server": os.environ["HTTPS_PROXY"], "bypass": "127.0.0.1,localhost"}   # remote hosts (compare mode) go through the runner proxy, the local server does not
    kw["args"] = ["--ignore-certificate-errors"]   # the h2 server of --http h2 has a self-signed certificate
    return p.chromium.launch(**kw)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--site", default=str(ROOT / "site"))
    ap.add_argument("--upstream", default="https://freeesta.github.io/mzlab/")
    ap.add_argument("--prime", action="store_true", help="visit once with the live site as fallback so the engine .wasm files land in --site")
    ap.add_argument("--profiles", default="none,fast4g,slow4g")
    ap.add_argument("--cpus", default="1,4")
    ap.add_argument("--modes", default="cold,warm")
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--http", default="h2", choices=["h2", "h1"], help="protocol of the local server: h2 like GitHub Pages (default) or HTTP/1.1 (6 connections)")
    ap.add_argument("--trace", action="store_true", help="also record a Chrome trace on the first cold run of each combination")
    ap.add_argument("--compare", nargs="*")
    ap.add_argument("--report", metavar="DIR", help="print the markdown tables of the run-*.json files of DIR")
    ap.add_argument("--ref", default="fast4g,1,cold", help="reference run of --report: profile,cpu,visit")
    ap.add_argument("--tables", action="store_true", help="--report: also the full request table of the reference run")
    ap.add_argument("--out", default="payload-out")
    a = ap.parse_args()
    if a.report:
        return rap.report(a.report, a.ref, a.tables)
    from playwright.sync_api import sync_playwright
    out = Path(a.out); out.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as p:
        if a.compare:
            b = launch(p); res = []
            for url in a.compare:
                for prof in a.profiles.split(","):
                    for cpu in map(int, a.cpus.split(",")):
                        runs = [rap.run_compare(b, url, prof, cpu) for _ in range(a.runs)]
                        med = {k: (statistics.median([r[k] for r in runs]) if isinstance(runs[0][k], (int, float)) else runs[0][k]) for k in runs[0]}
                        print(json.dumps(med), flush=True); res.append({"median": med, "runs": runs})
            (out / "compare.json").write_text(json.dumps(res, indent=1)); return
        if a.prime:
            srv, cls, base = serve(a.site, "none", a.upstream)
            b = launch(p); v = run_visit(b, base, "cold", 1); b.close(); srv.shutdown()
            print("primed:", len(list(Path(a.site).rglob("*.wasm"))), ".wasm files in", a.site, flush=True)
        sets = static_sets(a.site)
        for prof in a.profiles.split(","):
            for cpu in map(int, a.cpus.split(",")):
                for mode in a.modes.split(","):
                    runs = []
                    for i in range(a.runs):
                        if a.http == "h2":
                            stop, getlog, base = serve_h2(a.site, prof)
                        else:
                            srv, cls, base = serve(a.site, prof); stop = srv.shutdown; getlog = lambda: cls.log
                        b = launch(p)
                        tp = str(out / f"trace-{prof}-cpu{cpu}-{mode}.json") if (a.trace and i == 0 and mode == "cold") else None
                        try:
                            v = run_visit(b, base, mode, cpu, tp)
                            r = analyse(v, getlog(), sets, a.site)
                            if tp:
                                pg0 = v["pages"][0]
                                r["trace"] = trace_breakdown(tp, r["ms"]["ready"], r["ms"]["fcp"], pg0["t0"], None)
                                os.remove(tp)
                                if r["trace"]:
                                    top = r["trace"].pop("_top", [])
                                    for lab in ("fcp", "ready"):
                                        if r["ms"].get(lab):
                                            r["trace"]["split_" + lab] = idle_split(r["table"], top, r["ms"][lab])
                            r["t0"] = v["pages"][0]["t0"]
                            runs.append(r)
                            print(prof, cpu, mode, i, json.dumps(r["ms"]), json.dumps(r["chain"]), flush=True)
                        except Exception as e:
                            print(prof, cpu, mode, i, "FAILED", repr(e)[:200], flush=True)
                        b.close(); stop()
                    if runs:
                        (out / f"run-{prof}-cpu{cpu}-{mode}.json").write_text(json.dumps(runs, indent=1))


if __name__ == "__main__":
    main()
