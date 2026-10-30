"""Probe of page-load and click latency of the published site: first visit (cold), repeat visit (warm), first example file, typical clicks,
«New session → Full reset» and the reloads that follow. Measures, never asserts: used by docs/studi/VELOCITA_RICARICA.md.
Targets: the live GitHub Pages site (--url) or a local mirror of it with GitHub-Pages-like headers (--mirror DIR: files missing from DIR are fetched
from --url once and kept). Profiles: none, fast4g (9 Mbps down, --rtt ms added), cpu4 (CPU x4), slow (both); throttling is applied over CDP to the page,
its workers and the service worker. Output: one JSON per run (--out) with per-navigation marks, resources, long tasks and event timings.
"""
from __future__ import annotations

import argparse, email.utils, gzip, hashlib, json, mimetypes, os, threading, time, urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

# Fast 4G bandwidth (9 Mbps down, 1.5 up) with no added latency: the path from the cloud runner to GitHub Pages already has ~170 ms per request,
# the latency of DevTools' Fast 4G (165 ms); against the local mirror (--mirror) add it with --rtt 165
PROFILES = {"none": (None, 1), "fast4g": ((0, 9_000_000 / 8, 1_500_000 / 8), 1), "cpu4": (None, 4), "slow": ((0, 9_000_000 / 8, 1_500_000 / 8), 4)}

# runs in every page before its own scripts: paint, LCP, long tasks, event timings, readiness marks; a snapshot goes to localStorage on pagehide
PROBE = r"""(() => {
  const T0 = performance.timeOrigin, abs = t => Math.round(T0 + t);
  try { performance.setResourceTimingBufferSize(5000); } catch (_) {}
  const v = window.__v = { t0: Math.round(T0), url: location.href, lt: [], ev: [], marks: {}, phases: [] };
  const obs = (type, f, o) => { try { new PerformanceObserver(l => l.getEntries().forEach(f)).observe(Object.assign({ type, buffered: true }, o || {})); } catch (_) {} };
  obs("longtask", e => v.lt.push([abs(e.startTime), Math.round(e.duration)]));
  obs("paint", e => { v.marks[e.name === "first-paint" ? "fp" : "fcp"] = abs(e.startTime); });
  obs("largest-contentful-paint", e => { v.marks.lcp = abs(e.startTime); v.lcpEl = e.element ? (e.element.id || e.element.tagName) : ""; });
  obs("event", e => v.ev.push([abs(e.startTime), e.name, Math.round(e.duration), Math.round(e.processingStart - e.startTime), Math.round(e.processingEnd - e.processingStart), e.target ? (e.target.id || String(e.target.className || "") || e.target.tagName) : ""]), { durationThreshold: 16 });
  const m = v.marks; let ph = "";
  const poll = () => {
    const n = abs(performance.now());
    if (!m.drop) { const d = document.getElementById("drop"); if (d && d.offsetParent && d.getBoundingClientRect().height > 0) m.drop = n; }
    if (!m.ready && window.qqReady === true) m.ready = n;
    if (!m.coi && window.crossOriginIsolated) m.coi = n;
    if (!m.ctrl && navigator.serviceWorker && navigator.serviceWorker.controller) m.ctrl = n;
    if (!m.panel && document.querySelector("#dpanels .pnl canvas")) m.panel = n;
    const p = window.qqPhaseCur ? window.qqPhaseCur.key : ""; if (p !== ph) { ph = p; v.phases.push([n, p]); }
    setTimeout(poll, 16);
  };
  poll();
  v.snap = () => {
    const nav = performance.getEntriesByType("navigation")[0] || {};
    v.nav = { type: nav.type, ttfb: abs(nav.responseStart || 0), reqStart: abs(nav.requestStart || 0), workerStart: nav.workerStart ? abs(nav.workerStart) : 0, dcl: abs(nav.domContentLoadedEventEnd || 0), load: abs(nav.loadEventEnd || 0), proto: nav.nextHopProtocol, transfer: nav.transferSize, status: nav.responseStatus };
    v.res = performance.getEntriesByType("resource").map(e => [e.name.replace(location.origin, ""), e.initiatorType, abs(e.startTime), Math.round(e.duration), e.transferSize, e.encodedBodySize, e.decodedBodySize, e.renderBlockingStatus || "", e.nextHopProtocol, e.responseStatus || 0, e.workerStart > 0 ? 1 : 0, Math.round(e.responseStart ? e.responseStart - e.startTime : 0)]);
    return v;
  };
  addEventListener("pagehide", () => { try { const a = JSON.parse(localStorage.getItem("__probe") || "[]"); a.push(v.snap()); localStorage.setItem("__probe", JSON.stringify(a)); } catch (_) {} });
  try { localStorage.setItem("mzlab.tour.lr", JSON.stringify({ stato: "saltato" })); localStorage.setItem("mzlab.barra.hint", "1"); localStorage.setItem("mzlab.pannelli.affianca.hint", "1"); } catch (_) {}
})();"""

USABLE = "!!(window.__v && __v.marks.ready && __v.marks.drop && window.crossOriginIsolated)"
# after «Full reset» the page may stay without cross-origin isolation (the reload of browser.js isolateOnce does not come): usable without it
USABLE_ANY = "!!(window.__v && __v.marks.ready && __v.marks.drop)"


class Mirror(BaseHTTPRequestHandler):
    """Static mirror with the headers GitHub Pages sends (ETag, Last-Modified, Cache-Control max-age=600, gzip for text)."""
    protocol_version = "HTTP/1.1"
    root: Path = Path(".")
    upstream = ""
    lock = threading.Lock()
    gz: dict = {}

    def log_message(self, *a):
        pass

    def _file(self, rel: str) -> Path | None:
        p = self.root / (rel or "index.html")
        if rel.endswith("/") or rel == "":
            p = self.root / rel / "index.html"
        if p.is_file():
            return p
        if not self.upstream or p.with_name(p.name + ".404").exists():
            return None
        try:
            with urllib.request.urlopen(self.upstream + rel, timeout=120) as r:
                data = r.read()
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(data)
            return p
        except Exception:
            p.parent.mkdir(parents=True, exist_ok=True)
            p.with_name(p.name + ".404").write_bytes(b"")
            return None

    def do_GET(self):
        rel = self.path.split("?", 1)[0].split("#", 1)[0].lstrip("/")
        rel = rel[len("mzlab/"):] if rel.startswith("mzlab/") else rel
        p = self._file(rel)
        if p is None:
            self.send_response(404); self.send_header("Content-Length", "0"); self.end_headers(); return
        st = p.stat()
        etag = 'W/"%x-%x"' % (int(st.st_mtime), st.st_size)
        if self.headers.get("If-None-Match") == etag:
            self.send_response(304); self.send_header("ETag", etag); self.send_header("Cache-Control", "max-age=600"); self.end_headers(); return
        ctype = mimetypes.guess_type(p.name)[0] or "application/octet-stream"
        if p.suffix in (".js", ".mjs"):
            ctype = "application/javascript"
        if p.suffix == ".wasm":
            ctype = "application/wasm"
        data = p.read_bytes()
        enc = None
        if "gzip" in self.headers.get("Accept-Encoding", "") and (ctype.startswith("text/") or ctype in ("application/javascript", "application/json", "application/wasm", "image/svg+xml")):
            with self.lock:
                key = (str(p), etag)
                if key not in self.gz:
                    self.gz[key] = gzip.compress(data, 6)
                data = self.gz[key]
            enc = "gzip"
        self.send_response(200)
        self.send_header("Content-Type", ctype + ("; charset=utf-8" if ctype.startswith("text/") or ctype == "application/javascript" else ""))
        self.send_header("Content-Length", str(len(data)))
        self.send_header("ETag", etag)
        self.send_header("Last-Modified", email.utils.formatdate(st.st_mtime, usegmt=True))
        self.send_header("Cache-Control", "max-age=600")
        self.send_header("Access-Control-Allow-Origin", "*")
        if enc:
            self.send_header("Content-Encoding", enc); self.send_header("Vary", "Accept-Encoding")
        self.end_headers()
        self.wfile.write(data)


class Throttle:
    """CDP throttling on every target of the browser (pages, workers, service worker) through one browser session (non-flat attach)."""

    def __init__(self, browser, profile: str, rtt: int = 0):
        self.net, self.cpu = PROFILES[profile]
        if self.net and rtt:
            self.net = (rtt,) + self.net[1:]
        self.on = bool(self.net) or self.cpu > 1
        self.s = browser.new_browser_cdp_session()
        self.n = 0
        # what the service worker really asks the network (status 200/304, bytes, wall time): the page only sees the worker's answers
        self.sw: list = []
        self._pending: dict = {}
        self.s.on("Target.targetCreated", self._created)
        self.s.on("Target.attachedToTarget", self._attached)
        self.s.on("Target.receivedMessageFromTarget", self._message)
        self.s.send("Target.setDiscoverTargets", {"discover": True})

    def _created(self, ev):
        ti = ev["targetInfo"]
        if ti["type"] == "service_worker" or (self.on and ti["type"] in ("page", "worker", "shared_worker")):
            try:
                self.s.send("Target.attachToTarget", {"targetId": ti["targetId"], "flatten": False})
            except Exception:
                pass

    def _send(self, sid, method, params):
        self.n += 1
        try:
            self.s.send("Target.sendMessageToTarget", {"sessionId": sid, "message": json.dumps({"id": self.n, "method": method, "params": params})})
        except Exception:
            pass

    def _attached(self, ev):
        sid, kind = ev["sessionId"], ev["targetInfo"]["type"]
        if kind == "service_worker":
            self._send(sid, "Network.enable", {})
        if self.net:
            rtt, down, up = self.net
            if kind != "service_worker":
                self._send(sid, "Network.enable", {})
            self._send(sid, "Network.emulateNetworkConditions", {"offline": False, "latency": rtt, "downloadThroughput": down, "uploadThroughput": up})
        if self.cpu > 1 and kind in ("page", "worker"):
            self._send(sid, "Emulation.setCPUThrottlingRate", {"rate": self.cpu})
        self._send(sid, "Runtime.runIfWaitingForDebugger", {})

    def _message(self, ev):
        try:
            m = json.loads(ev["message"])
        except Exception:
            return
        k, pr = m.get("method"), m.get("params") or {}
        if k == "Network.requestWillBeSent":
            self._pending[pr["requestId"]] = [round(time.time() * 1000), pr["request"]["url"], 0, 0, 0]
        elif k == "Network.responseReceived" and pr.get("requestId") in self._pending:
            self._pending[pr["requestId"]][2] = pr["response"]["status"]
        elif k == "Network.loadingFinished" and pr.get("requestId") in self._pending:
            r = self._pending.pop(pr["requestId"]); r[3] = pr.get("encodedDataLength", 0); r[4] = round(time.time() * 1000); self.sw.append(r)


CLICKS = [("nav-draw", "#nav button[data-v=draw]"), ("nav-data", "#nav button[data-v=data]"), ("settings", "#np-set"), ("settings-close", "Escape"),
          ("sample-table", "#samptab"), ("sample-back", "#samptab"), ("file-row", "#flst .fl .nm >> nth=0")]
CLICK_DATA = """t => ({ ev: __v.ev.filter(e => e[0] >= t - 50), lt: __v.lt.filter(e => e[0] >= t - 50),
  res: performance.getEntriesByType('resource').filter(e => performance.timeOrigin + e.startTime >= t).map(e => [e.name.replace(location.origin, ''), Math.round(e.duration), e.decodedBodySize]) })"""


def t_wall() -> int:
    return round(time.time() * 1000)


class Run:
    """One browser context through the steps; every step leaves its marks, requests and page snapshots in `out`."""

    def __init__(self, b, thr, base: str, out_path: str, no_sw: bool = False):
        self.thr, self.base, self.out_path = thr, base, out_path
        self.out = {"base": base, "steps": {}}
        self.navs, self.reqs, self.t_step = [], [], 0
        c = b.new_context(viewport={"width": 1400, "height": 900}, service_workers="block" if no_sw else "allow")
        self.usable = USABLE_ANY if no_sw else USABLE
        c.set_default_timeout(300000)
        c.add_init_script(PROBE)
        c.on("request", lambda r: self.reqs.append([t_wall(), r.url.replace(base, ""), r.resource_type, r.is_navigation_request()]))
        self.pg = c.new_page()
        self.pg.on("framenavigated", lambda fr: self.navs.append([t_wall(), fr.url.replace(base, "")]) if fr == self.pg.main_frame else None)

    def until_true(self, expr: str, timeout_s: float = 300.0) -> None:
        # the page CSP has no 'unsafe-eval', so Playwright's wait_for_function cannot run: an in-page poll through evaluate (CDP), retried across navigations
        poll = f"new Promise(ok => {{ const end = Date.now() + 1000, f = () => {{ let v = false; try {{ v = !!({expr}); }} catch (_) {{}} if (v || Date.now() > end) ok(v); else setTimeout(f, 30); }}; f(); }})"
        end = time.time() + timeout_s
        while time.time() < end:
            try:
                if self.pg.evaluate(poll):
                    return
            except Exception:
                pass
        raise TimeoutError(expr)

    def quiet(self, ms: int = 1500, cap: int = 30000) -> None:
        """No long task for `ms` (or `cap` reached)."""
        try:
            self.until_true(f"(() => {{ const n = performance.timeOrigin + performance.now(), l = __v.lt[__v.lt.length - 1]; return !l || n - (l[0] + l[1]) > {ms}; }})()", cap / 1000)
        except TimeoutError:
            pass

    def snaps(self) -> list:
        pg = self.pg
        prev = pg.evaluate("JSON.parse(localStorage.getItem('__probe') || '[]')")
        cur = pg.evaluate("__v.snap()")
        pg.evaluate("localStorage.removeItem('__probe')")
        # only the pages of this step (the page left by the previous step writes its snapshot when it goes away)
        return [x for x in prev if x.get("t0", 0) >= self.t_step - 5 and str(x.get("url", "")).startswith("http")] + [cur]

    def step(self, name: str, act, until: str = "") -> None:
        until = until or self.usable
        n0, r0, sw0 = len(self.navs), len(self.reqs), len(self.thr.sw)
        t0 = self.t_step = t_wall()
        if self.pg.url.startswith("http"):
            self.pg.evaluate("localStorage.removeItem('__probe')")
        act()
        # the page of this step only (a reload or a reset navigates)
        self.until_true(f"!!window.__v && __v.t0 >= {t0 - 5} && ({until})" if until in (USABLE, USABLE_ANY) else until)
        t1 = t_wall()
        self.quiet()
        st = {"t0": t0, "usable": t1, "navs": self.navs[n0:], "reqs": self.reqs[r0:], "pages": self.snaps(), "settled": t_wall()}
        st["sw"] = [[r[0], r[1].replace(self.base, ""), r[2], r[3], r[4]] for r in self.thr.sw[sw0:]]
        self.out["steps"][name] = st
        self.save()
        print(f"{name}: usable {t1 - t0} ms, {len(self.navs) - n0} navigations, {len(self.reqs) - r0} requests", flush=True)

    def save(self) -> None:
        Path(self.out_path).write_text(json.dumps(self.out))

def demo(self: Run, example: str) -> None:
    """The first example file, from the request to the first plot."""
    pg = self.pg
    pg.evaluate(f"void upload([], 'data', [{json.dumps(example)}])")
    # without the landing that opens the first file by itself, «Load data» (#opbtn) is clicked as soon as it can be
    self.until_true("!!document.querySelector('#dpanels .pnl canvas') || !!(document.querySelector('#opbtn:not([disabled])') && document.querySelector('#opbtn').offsetParent)")
    if not pg.evaluate("!!document.querySelector('#dpanels .pnl canvas')"):
        pg.click("#opbtn")

def clicks(self: Run) -> None:
    """Typical clicks, one at a time: event timings, long tasks and fetches from the click until the page is quiet."""
    pg, res = self.pg, {}
    for name, sel in CLICKS:
        t0 = t_wall()
        try:
            if sel == "Escape":
                pg.keyboard.press("Escape")
            elif not pg.locator(sel).first.is_visible():
                res[name] = {"skipped": "not visible"}; continue
            else:
                pg.locator(sel).first.click(timeout=5000)
            pg.evaluate("new Promise(ok => requestAnimationFrame(() => requestAnimationFrame(ok)))")
            self.quiet(800, 15000)
        except Exception as e:
            res[name] = {"error": str(e)[:200]}; continue
        v = pg.evaluate(CLICK_DATA, t0)
        v["wall"] = t_wall() - t0
        res[name] = v
        print(f"click {name}: events {[(e[1], e[2]) for e in v['ev']][:4]} long tasks {[e[1] for e in v['lt']][:6]} fetched {len(v['res'])}", flush=True)
    self.out["clicks"] = res

def reset(self: Run) -> None:
    """«New session → Full reset», as the user does it."""
    self.pg.evaluate("QQRESET.ask()")
    self.pg.click("#resetdlg button[data-reset=all]")


def args():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", default="https://freeesta.github.io/mzlab/")
    ap.add_argument("--mirror", help="serve a local mirror from this folder (filled from --url on demand)")
    ap.add_argument("--profile", default="none", choices=sorted(PROFILES))
    ap.add_argument("--rtt", type=int, default=0, help="latency added by the network profiles, ms")
    ap.add_argument("--out", required=True)
    ap.add_argument("--no-sw", action="store_true", help="block the service worker (no offline copy, no cross-origin isolation): the cost of the worker itself")
    ap.add_argument("--example", default="FullScan_t15.mzML")
    return ap.parse_args()


def serve_mirror(folder: str, upstream: str) -> str:
    Mirror.root = Path(folder); Mirror.root.mkdir(parents=True, exist_ok=True); Mirror.upstream = upstream
    srv = ThreadingHTTPServer(("127.0.0.1", 0), Mirror)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{srv.server_address[1]}/mzlab/"


def main():
    a = args()
    from playwright.sync_api import sync_playwright
    base = serve_mirror(a.mirror, a.url) if a.mirror else a.url
    exe = "/opt/pw-browsers/chromium"
    proxy = {"server": os.environ["HTTPS_PROXY"]} if os.environ.get("HTTPS_PROXY") and not a.mirror else None
    with sync_playwright() as p:
        b = p.chromium.launch(proxy=proxy, **({"executable_path": exe} if os.path.isfile(exe) else {}))
        r = Run(b, Throttle(b, a.profile, a.rtt), base, a.out, a.no_sw)
        r.out["profile"] = a.profile
        r.step("cold", lambda: r.pg.goto(base + "#/data"))
        r.step("warm", lambda: r.pg.reload())
        try:
            r.step("file", lambda: demo(r, a.example), "!!document.querySelector('#dpanels .pnl canvas')")
        except TimeoutError:
            print("file: no plot", flush=True)
        clicks(r)
        r.step("reset", lambda: reset(r), USABLE_ANY)
        r.step("warm-after-reset", lambda: r.pg.reload(), USABLE_ANY)
        r.save()
        b.close()


if __name__ == "__main__":
    main()
