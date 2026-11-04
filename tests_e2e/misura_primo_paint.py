"""Page-load probe of the built site (first paint to usable), for docs/agenti/misure-velocita.md: serves `site/` with GitHub-Pages-like headers
(tests_e2e/misura_ricarica.py Mirror) and measures, per scenario and as the median of --runs: FCP, LCP, CLS, long tasks until usable, requests and
bytes before FCP and until usable, render-blocking files, engine ready, console warnings. Measures, never asserts (the guard is e2e_budget_avvio.py).

    python3 tests_e2e/misura_primo_paint.py --site site [--runs 5] [--scenari cold,warm,...] [--out run.json] [--browser chromium|webkit]

Scenarios: cold (new context, first visit), warm (reload served by the service worker), en (cold with ?lang=en), dark (warm with dark theme saved),
cpu4 / cpu4warm (CPU x4), fast4g (150 ms + 9 Mbps, cold), ipad (iPad viewport and touch, cold), draw/flow/tabui (warm, then click the view).
"""
import argparse, json, statistics, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
from misura_ricarica import Mirror, Throttle, serve_mirror  # noqa: E402,F401

PROBE = r"""(() => {
  const T0 = performance.timeOrigin;
  try { performance.setResourceTimingBufferSize(5000); } catch (_) {}
  const v = window.__p = { lt: [], cls: 0, shifts: [], m: {}, warn: [] };
  const obs = (type, f, o) => { try { new PerformanceObserver(l => l.getEntries().forEach(f)).observe(Object.assign({ type, buffered: true }, o || {})); } catch (_) {} };
  obs("longtask", e => v.lt.push([Math.round(e.startTime), Math.round(e.duration)]));
  obs("paint", e => { v.m[e.name === "first-paint" ? "fp" : "fcp"] = Math.round(e.startTime); });
  obs("largest-contentful-paint", e => { v.m.lcp = Math.round(e.startTime); v.lcpEl = e.element ? (e.element.id || e.element.tagName) : ""; });
  obs("layout-shift", e => { if (e.hadRecentInput) return; v.cls += e.value; v.shifts.push([Math.round(e.startTime), +e.value.toFixed(4), (e.sources || []).map(s => s.node ? (s.node.id || s.node.className || s.node.tagName) : "").join(" ").slice(0, 80)]); });
  const poll = () => {
    const n = Math.round(performance.now());
    if (!document.documentElement) { setTimeout(poll, 8); return; }
    if (!v.m.drop) { const d = document.getElementById("drop"); if (d && d.offsetParent && d.getBoundingClientRect().height > 0) v.m.drop = n; }
    if (!v.m.ready && window.qqReady === true) v.m.ready = n;
    if (!v.m.i18n && !document.documentElement.classList.contains("i18n-wait")) v.m.i18n = n;
    if (!v.m.ready || !v.m.drop) setTimeout(poll, 8);
  };
  poll();
  v.snap = () => {
    const nav = performance.getEntriesByType("navigation")[0] || {};
    v.nav = { ttfb: Math.round(nav.responseStart || 0), dcl: Math.round(nav.domContentLoadedEventEnd || 0), load: Math.round(nav.loadEventEnd || 0) };
    v.res = performance.getEntriesByType("resource").map(e => [e.name.replace(location.origin, ""), Math.round(e.startTime), Math.round(e.responseEnd), e.transferSize, e.decodedBodySize, e.renderBlockingStatus || ""]);
    v.coi = !!window.crossOriginIsolated; v.lang = document.documentElement.lang; v.theme = document.documentElement.getAttribute("data-theme");
    return JSON.parse(JSON.stringify(v));
  };
  try { localStorage.setItem("mzlab.tour.lr", JSON.stringify({ stato: "saltato" })); localStorage.setItem("mzlab.barra.hint", "1"); } catch (_) {}
})();"""

VIEWS = {"draw": ("#nav button[data-v=draw]", "#v-draw:not([hidden])"), "flow": ("#nav button[data-v=flow]", "#v-flow:not([hidden]), #flowview:not([hidden])"),
         "tabui": ("#nav button[data-v=tpmine]", "#v-tpmine:not([hidden]), #tpmine:not([hidden])")}


def poll(pg, expr, timeout=120.0):
    js = f"new Promise(ok => {{ const end = Date.now() + 1000, f = () => {{ let v = false; try {{ v = !!({expr}); }} catch (_) {{}} if (v || Date.now() > end) ok(v); else setTimeout(f, 20); }}; f(); }})"
    end = time.time() + timeout
    while time.time() < end:
        try:
            if pg.evaluate(js):
                return True
        except Exception:
            pass
    raise TimeoutError(expr)


def settle(pg, ms=1200, cap=20.0):
    """No long task for `ms` after usable (or cap), so long tasks of the boot tail are counted."""
    try:
        poll(pg, f"(() => {{ const n = performance.now(), l = __p.lt[__p.lt.length - 1]; return !l || n - (l[0] + l[1]) > {ms}; }})()", cap)
    except TimeoutError:
        pass


def summary(s):
    fcp = s["m"].get("fcp") or s["m"].get("fp") or 0
    usable = max(s["m"].get("ready", 0), s["m"].get("drop", 0))
    pre = [r for r in s["res"] if r[1] <= fcp]
    upto = [r for r in s["res"] if r[1] <= usable]
    return {"ttfb": s["nav"]["ttfb"], "fcp": fcp, "lcp": s["m"].get("lcp", 0), "i18n": s["m"].get("i18n", 0), "drop": s["m"].get("drop", 0), "ready": s["m"].get("ready", 0),
            "cls": round(s["cls"], 4), "lt_n": len([x for x in s["lt"] if x[0] <= usable + 2000]), "lt_max": max([x[1] for x in s["lt"] if x[0] <= usable + 2000] or [0]),
            "lt_sum": sum(x[1] for x in s["lt"] if x[0] <= usable + 2000),
            "req_fcp": len(pre), "kb_fcp": round(sum(r[4] for r in pre) / 1024), "req_ready": len(upto), "kb_ready": round(sum(r[4] for r in upto) / 1024),
            "wire_kb_ready": round(sum(r[3] for r in upto) / 1024), "blocking": len([r for r in s["res"] if r[5] == "blocking"]), "coi": s["coi"], "lang": s["lang"]}


def one(b, base, scen, browser_name):
    prof = {"cpu4": "cpu4", "cpu4warm": "cpu4", "fast4g": "fast4g"}.get(scen, "none")
    thr = Throttle(b, prof, 150 if prof == "fast4g" else 0) if browser_name == "chromium" else None
    kw = {"viewport": {"width": 1400, "height": 900}}
    if scen == "ipad":
        kw = {"viewport": {"width": 1024, "height": 1366}, "is_mobile": True, "has_touch": True, "device_scale_factor": 2,
              "user_agent": "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1"}
    c = b.new_context(**kw)
    c.add_init_script(PROBE)
    warn = []
    pg = c.new_page()
    pg.on("console", lambda m: warn.append(f"{m.type}: {m.text[:160]}") if m.type in ("warning", "error") else None)
    url = base + ("?lang=en" if scen == "en" else "") + "#/data"
    usable = "!!(window.__p && __p.m.ready && __p.m.drop)"
    pg.goto(url)
    poll(pg, usable)
    warm = scen in ("warm", "dark", "cpu4warm") or scen in VIEWS
    if warm:
        # the service worker fills its cache in the background: wait for its end mark, then measure the reload
        settle(pg, 1500, 30)
        try:
            poll(pg, "caches.keys().then(k => Promise.all(k.filter(n => n.startsWith('mzlab-app-')).map(n => caches.open(n).then(c => c.match('.fill-done'))))).then(a => a.some(Boolean))", 60)
        except TimeoutError:
            pass
        if scen == "dark":
            pg.evaluate("(() => { const k = NOMI.KEY('prefs'); const p = JSON.parse(localStorage.getItem(k) || '{}'); p.theme = 'dark'; localStorage.setItem(k, JSON.stringify(p)); })()")
        warn.clear()
        pg.reload()
        poll(pg, usable)
    settle(pg)
    s = summary(pg.evaluate("__p.snap()"))
    if scen in VIEWS:
        btn, shown = VIEWS[scen]
        t0 = pg.evaluate("performance.now()")
        pg.click(btn)
        poll(pg, f"!!document.querySelector('{shown}')", 60)
        t1 = pg.evaluate("performance.now()")
        settle(pg, 1000, 20)
        lt = pg.evaluate(f"__p.lt.filter(x => x[0] >= {t0})")
        n = pg.evaluate(f"performance.getEntriesByType('resource').filter(e => e.startTime >= {t0}).map(e => e.decodedBodySize)")
        s.update({"view_ms": round(t1 - t0), "view_lt_max": max([x[1] for x in lt] or [0]), "view_req": len(n), "view_kb": round(sum(n) / 1024)})
    s["warn"] = warn[:20]
    c.close()
    return s


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--site", default=str(ROOT / "site"))
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--scenari", default="cold,warm,en,dark,cpu4,cpu4warm,fast4g,ipad,draw,flow,tabui")
    ap.add_argument("--browser", default="chromium")
    ap.add_argument("--out")
    a = ap.parse_args()
    from playwright.sync_api import sync_playwright
    base = serve_mirror(a.site, "")
    out = {}
    with sync_playwright() as p:
        bt = getattr(p, a.browser)
        for scen in a.scenari.split(","):
            runs = []
            for _ in range(a.runs):
                exe = "/opt/pw-browsers/chromium"
                b = bt.launch(**({"executable_path": exe} if a.browser == "chromium" and Path(exe).is_file() else {}))
                try:
                    runs.append(one(b, base, scen, a.browser))
                except Exception as e:
                    print(f"{scen}: {e}", flush=True)
                b.close()
            if not runs:
                continue
            med = {k: (statistics.median([r[k] for r in runs]) if isinstance(runs[0][k], (int, float)) and not isinstance(runs[0][k], bool) else runs[0][k]) for k in runs[0] if k != "warn"}
            med["warn"] = sorted({w for r in runs for w in r["warn"]})
            out[scen] = {"median": med, "runs": runs}
            print(scen, json.dumps({k: v for k, v in med.items() if k != "warn"}), flush=True)
            for w in med["warn"]:
                print("   ", w, flush=True)
    if a.out:
        Path(a.out).write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
