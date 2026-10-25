"""Speed of the main actions on a SLOW computer: the built site (Rust/WASM engine, Chromium) with the CPU slowed 1x / 4x / 6x and
navigator.hardwareConcurrency set to 2 and 4, on the example files (Full Scan demo, mzlab/web/esempi). A report, not a test: nothing here fails on a number
and the script is not in the CI. Not in the normal test run (name without e2e_).

    python3 tools/prepara_sito.py                       # once: site/ with the engine
    python3 tests_e2e/misura_macchine.py --md docs/agenti/misure-macchine.md [--json OUT.json] [--reps 12] [--rates 1,4,6] [--cores 2,4]

Actions (time from the event of the user to the end of the repaint that shows the result, measured inside the page): open the demo (first chart,
ready), drag the vertical line of the chromatogram with the linked MS1 spectrum (frames, long tasks, time per update), change of panel and of file, 2D map,
3D map (when the button exists), XIC from one click (p50 / p95 as in V-0), peak of the JS heap and of the engine memory. Phases (request / engine / JSON /
drawing) come from hooks that wrap fetch, Response.json and draw(); nothing of the product is changed."""
import sys, os, json, time, subprocess, argparse; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

INIT = r"""(() => {
  const V = window.__M = { fe: [], js: [], dr: [], wmem: 0, heap: 0, fr: null, lt: [] };
  const W = window.Worker;
  window.Worker = class extends W { constructor(u, o) { super(u, o); this.addEventListener('message', e => { const m = e.data; if (m && m.mem) V.wmem = Math.max(V.wmem, m.mem); }); } };
  const j0 = Response.prototype.json; Response.prototype.json = function () { const t0 = performance.now(); return j0.call(this).then(x => { V.js.push({ t0, t1: performance.now() }); return x; }); };
  V.wrap = () => { if (V.wrapped) return; V.wrapped = true;
  const f0 = window.fetch;
    window.fetch = function (u, ...a) { const s = String(u), r = { u: s, t0: performance.now() }; if (/api\//.test(s)) V.fe.push(r);
      return f0.call(this, u, ...a).then(x => { r.t1 = performance.now(); r.ms = +(x.headers.get('X-Py-Ms') || 0); return x; }, e => { r.t1 = performance.now(); throw e; }); };
    for (const n of ['draw', 'drawMap']) { const d0 = window[n]; if (typeof d0 !== 'function') continue;
      window[n] = async function (p) { const t0 = performance.now(); try { return await d0.apply(this, arguments); } finally { V.dr.push({ t0, t1: performance.now() }); } }; } };
  const heap = () => { const h = (performance.memory || {}).usedJSHeapSize || 0; if (h > V.heap) V.heap = h; };
  setInterval(heap, 200);
  // run `act`, wait until `until()` is true and the next frame is painted: ms from before the action to after the paint, with the phases
  V.run = (act, until, cap) => new Promise(ok => {
    const t0 = performance.now(), i0 = [V.fe.length, V.js.length, V.dr.length]; act();
    const fin = t1 => { requestAnimationFrame(() => requestAnimationFrame(() => { const t2 = performance.now(), W = r => r.t0 >= t0 && r.t1 <= t2 + 5; heap();
      const fe = V.fe.slice(i0[0]).filter(r => r.t1 && W(r)), js = V.js.slice(i0[1]).filter(W), dr = V.dr.slice(i0[2]).filter(W), sum = a => a.reduce((s, r) => s + (r.t1 - r.t0), 0);
      let net = 0, end = -1; for (const r of fe.slice().sort((a, b) => a.t0 - b.t0)) { const b = Math.max(r.t0, end); if (r.t1 > b) net += r.t1 - b; end = Math.max(end, r.t1); }
      ok({ ms: t2 - t0, req: net, eng: fe.reduce((s, r) => s + r.ms, 0), json: sum(js), draw: sum(dr), n: fe.length }); })); };
    const poll = () => { const n = performance.now(); if (until()) fin(n); else if (n - t0 > (cap || 120000)) ok(null); else requestAnimationFrame(poll); }; poll(); });
  V.frames = on => { if (on) { V.fr = []; V.lt = []; const t = a => { if (V.fr) { V.fr.push(a); requestAnimationFrame(t); } }; requestAnimationFrame(t);
      try { V.po = new PerformanceObserver(l => l.getEntries().forEach(e => V.lt.push(e.duration))); V.po.observe({ entryTypes: ['longtask'] }); } catch (_) {} return 0; }
    const fr = V.fr || []; V.fr = null; if (V.po) V.po.disconnect(); const g = []; for (let i = 1; i < fr.length; i++) g.push(fr[i] - fr[i - 1]); return { g, lt: V.lt }; };
})();"""
NEWPANEL = "(t, n) => E.panels.filter(p => p.type === t).length > n && E.panels.filter(p => p.type === t).every(p => p._a)"


def pct(v, q):
    v = sorted(v); return round(v[min(len(v) - 1, int(len(v) * q))], 1) if v else None


def phases(rows):
    """median of the phases over the repetitions (request = union of the api/ fetches, engine = sum of X-Py-Ms, JSON, drawing)"""
    rows = [r for r in rows if r]
    return {k: pct([r[k] for r in rows], .5) for k in ("req", "eng", "json", "draw")} if rows else {}


class Bench:
    def __init__(s, pg, reps): s.pg, s.reps, s.out = pg, reps, {}

    def run(s, act, until, arg=None, cap=120000):
        """(ms, phases) of one action; `act` / `until` are page functions given as source text"""
        return s.pg.evaluate(f"(a) => __M.run(() => ({act})(a), () => ({until})(a), {cap})", arg)

    def put(s, name, rows, extra=None):
        ok = [r for r in rows if r]
        s.out[name] = {"n": len(ok), "p50": pct([r["ms"] for r in ok], .5), "p95": pct([r["ms"] for r in ok], .95), **phases(ok), **(extra or {})}

    def frames_of(s, name, fn):
        s.pg.evaluate("__M.frames(true)"); t = time.time(); fn(); fr = s.pg.evaluate("__M.frames(false)"); dt = time.time() - t
        g = fr["g"]; lost = sum(round(x / 16.7) - 1 for x in g if x > 25)
        s.out[name] = {"n": len(g), "p50": pct(g, .5), "p95": pct(g, .95), "lost": lost, "long": len(fr["lt"]), "long_ms": round(sum(fr["lt"])), "secs": round(dt, 1)}

    def open_demo(s, port):
        pg = s.pg; pg.goto(f"http://127.0.0.1:{port}/"); attendi(pg, "window.MZLAB_LR !== undefined", timeout=240000); pg.wait_for_selector("#demobtn", timeout=240000)
        pg.evaluate("__M.wrap()")
        pg.click("#demobtn"); pg.click("#demochoice [data-demo=full]")
        pg.wait_for_function("document.querySelectorAll('#flist [data-k=time]').length===5", timeout=600000)       # the five files downloaded and listed
        pg.evaluate("window.__t0 = performance.now()")
        pg.evaluate("""() => { window.__first = null; const f = () => { if (!window.__first && (typeof E !== 'undefined' && E.panels || []).some(p => p._a)) window.__first = performance.now(); if (!window.__first) requestAnimationFrame(f); }; f(); }""")
        pg.click("text=Carica dati"); print("  loading", flush=True)
        pg.wait_for_function("window.__first", timeout=600000); first = pg.evaluate("__first - __t0")
        ready(pg, timeout=600000); done = pg.evaluate("performance.now() - __t0") - 400            # `ready` waits 400 ms of quiet
        s.out["open_first_chart"] = {"n": 1, "p50": round(first), "p95": round(first)}; s.out["open_ready"] = {"n": 1, "p50": round(done), "p95": round(done)}

    def drag(s):
        """the vertical line of the chromatogram (its dot) dragged across the run: the MS1 spectrum follows"""
        pg = s.pg; pg.evaluate("(p => p.el.scrollIntoView({block: 'center'}))(E.panels.find(p => p.type === 'chrom'))"); pg.wait_for_timeout(300)
        pg.mouse.click(*pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'chrom'), r = p.cv.getBoundingClientRect(); return [r.left + r.width * 0.5, r.top + r.height * 0.4]; })()")); frames(pg, 4)      # the click places the line; the mouse then grabs it
        c = pg.evaluate("(() => { const p = E.panels.find(q => q.type === 'chrom'), r = p.cv.getBoundingClientRect(); return [r.left + p._a.X(p.cur), r.top + r.height * 0.4, r.left, r.width]; })()")
        def sweep():
            pg.mouse.move(c[0], c[1]); pg.mouse.down()
            for k in range(2):
                for i in range(40): pg.mouse.move(c[2] + c[3] * (0.15 + 0.7 * (i if k == 0 else 39 - i) / 39), c[1]); pg.evaluate("1")      # a round trip per move = the pace of a hand
            pg.mouse.up()
        i0 = pg.evaluate("__M.dr.length"); s.frames_of("drag_line", sweep)
        du = pg.evaluate("i => __M.dr.slice(i).map(r => r.t1 - r.t0)", i0)
        s.out["drag_update"] = {"n": len(du), "p50": pct(du, .5), "p95": pct(du, .95)}

    def switches(s):
        rows = []
        for _ in range(s.reps):                                                    # a new chromatogram panel and its closing
            n = s.pg.evaluate("E.panels.filter(p => p.type === 'chrom').length")
            rows.append(s.run("() => document.getElementById('np-chrom').click()", f"() => ({NEWPANEL})('chrom', {n})"))
            s.pg.evaluate("(() => { const q = E.panels.filter(p => p.type === 'chrom'); closePanel(q[q.length - 1]); })()"); frames(s.pg, 3)
        s.put("switch_panel", rows)
        rows = []; ks = s.pg.evaluate("tabFiles('full').map(f => f.k)")                 # change of file: the click on another row of the list
        if not s.pg.is_visible("#flst"): s.pg.click("#stbar [data-cmd=file]")
        for i in range(s.reps):
            k = ks[(i + 1) % len(ks)]
            rows.append(s.run("k => document.querySelector(`#flst .fl[data-k='${k}'] .nm`).click()",
                              "k => !E.pending.length && E.panels.every(p => !p.el || p._a) && __M.fe.every(r => r.t1)", k))
            frames(s.pg, 3)
        s.put("switch_file", rows)

    def maps(s):
        pg = s.pg; rows = []
        for _ in range(max(3, s.reps // 3)):
            rows.append(s.run("() => document.getElementById('np-map').click()",
                              "() => E.panels.some(p => p.type === 'map' && p._a) && __M.fe.every(r => r.t1)"))
            pg.evaluate("(() => { const q = E.panels.filter(p => p.type === 'map'); q.forEach(closePanel); })()"); frames(pg, 3)
        s.put("map_2d", rows)
        pg.click("#np-map"); attendi(pg, "E.panels.some(p => p.type === 'map' && p._a)", timeout=600000); frames(pg, 5)
        pg.evaluate("E.panels.find(p => p.type === 'map').el.scrollIntoView({block: 'center'})"); pg.wait_for_timeout(300)
        if pg.locator(".pnl.map [data-o=view][data-v='3d']").count():
            rows = []
            for _ in range(max(3, s.reps // 3)):
                rows.append(s.run("() => document.querySelector(\".pnl.map [data-o=view][data-v='3d']\").click()",
                                  "() => !!(document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g && window.__m3d.V.Z)"))
                pg.keyboard.press("Escape"); pg.wait_for_function("!document.querySelector('dialog.m3d[open]')", timeout=60000)
            s.put("map_3d", rows)
        pg.evaluate("E.panels.filter(p => p.type === 'map').forEach(closePanel)"); frames(pg, 3)

    def xic(s):
        """XIC from one click on a labelled peak of the spectrum (G2 of V-0: every repetition is a new m/z, the page memoises per m/z)"""
        pg = s.pg; rows = []; i = 0
        pg.evaluate("(p => p.el.scrollIntoView({block: 'center'}))(E.panels.find(p => p.type === 'chrom'))")
        while len(rows) < s.reps and i < s.reps * 5:
            fr = (0.06 + 0.88 * ((i * 0.381966) % 1)); i += 1
            pg.evaluate("f => { const a = E.panels.find(p => p.type === 'chrom')._a; const r = E.panels.find(p => p.type === 'chrom').cv.getBoundingClientRect(); window.__cx = r.left + a.X(a.x0 + (a.x1 - a.x0) * f); window.__cy = r.top + r.height * 0.4; }", fr)
            pg.mouse.click(*pg.evaluate("[__cx, __cy]")); frames(pg, 4)
            ms = pg.evaluate("""(() => { const p = E.panels.find(q => q.type === 'spec' && q.tab === E.tab); return p && p._a ? [...new Set((p._a.lbls || []).filter(l => l.m != null).map(l => l.m))].slice(0, 2) : []; })()""")
            for m in ms:
                n = pg.evaluate("E.panels.filter(p => p.type === 'xic').length")
                rows.append(s.run("""m => { const p = E.panels.find(q => q.type === 'spec' && q.tab === E.tab), a = p._a;
                    AZIONI.esegui("spec.xic", { vista: "spec", p, a, m, has: true, lab: m.toFixed(mzd(p, p.level)), e: { preventDefault() {}, clientX: 0, clientY: 0 }, px: 0, py: 0, pol: a.data[0].f && a.data[0].f.polarity }); }""",
                                  f"() => ({NEWPANEL})('xic', {n})", m))
                pg.evaluate("E.panels.filter(p => p.type === 'xic').forEach(closePanel)"); frames(pg, 3)
        s.put("xic_click", rows)

    def memory(s):
        h = s.pg.evaluate("Math.round(__M.heap / 1048576 * 10) / 10"); w = s.pg.evaluate("Math.round(__M.wmem / 1048576)")
        ua = None
        try: ua = s.pg.evaluate("window.crossOriginIsolated && performance.measureUserAgentSpecificMemory ? performance.measureUserAgentSpecificMemory().then(r => Math.round(r.bytes / 1048576)) : null")
        except Exception: pass
        s.out["memory_peak"] = {"heap_mb": h, "engine_mb": w, "uas_mb": ua}


def one_config(p, port, rate, cores, reps):
    b = p.chromium.launch(args=["--enable-precise-memory-info", "--js-flags=--expose-gc"])
    ctx = b.new_context(viewport={"width": 1500, "height": 1000}, locale="it-IT", reduced_motion="reduce"); pg = ctx.new_page()
    pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.add_init_script(f"Object.defineProperty(navigator, 'hardwareConcurrency', {{get: () => {cores}}});")
    pg.add_init_script(INIT)
    ctx.new_cdp_session(pg).send("Emulation.setCPUThrottlingRate", {"rate": rate})
    errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    B = Bench(pg, reps); t = time.time()
    for name, fn in (("open", lambda: B.open_demo(port)), ("drag", B.drag), ("switch", B.switches), ("map", B.maps), ("xic", B.xic), ("memory", B.memory)):
        print("  step", name, flush=True)
        try: fn()
        except Exception as e: B.out.setdefault("errors", []).append(f"{name}: {str(e).splitlines()[0][:160]}")
    B.out["errors"] = B.out.get("errors", []) + errs[:3]; B.out["wall_s"] = round(time.time() - t)
    b.close(); return B.out


ROWS = [("open_first_chart", "open the Full Scan demo: first chart"), ("open_ready", "open the Full Scan demo: ready"), ("drag_line", "drag the line: frame interval (ms)"), ("drag_update", "drag the line: one draw() of chromatogram or spectrum"),
        ("switch_panel", "new panel (chromatogram)"), ("switch_file", "change of file"), ("map_2d", "2D map"), ("map_3d", "3D map"), ("xic_click", "XIC from one click")]


def mem(m): return f"{m.get('heap_mb', '–')} / {m.get('engine_mb', '–')}"


def table(res, rates, cores):
    cols = [(r, c) for r in rates for c in cores]
    L = ["| action | " + " | ".join(f"{r}x / {c} cores" for r, c in cols) + " |", "|---|" + "---|" * len(cols)]
    for k, name in ROWS:
        cell = lambda r, c: (lambda d: "–" if not d or d.get("p50") is None else f"{d['p50']:g} / {d['p95']:g}" + (f" ({d['lost']} lost)" if "lost" in d else ""))(res.get(f"{r}_{c}", {}).get(k))
        L.append(f"| {name} | " + " | ".join(cell(r, c) for r, c in cols) + " |")
    L.append("| JS heap peak / engine peak (MB) | " + " | ".join(mem(res.get(f"{r}_{c}", {}).get("memory_peak") or {}) for r, c in cols) + " |")
    return L


def worst(res, rate, cores):
    """the 5 slowest actions at the highest slowdown (p50) with where the time goes"""
    d = res.get(f"{rate}_{cores}", {}); acts = [(k, d[k]) for k, _ in ROWS if k in d and k not in ("drag_line", "drag_update") and d[k].get("p50") is not None]
    if d.get("drag_update", {}).get("p50") is not None: acts.append(("drag_update", d["drag_update"]))
    return sorted(acts, key=lambda a: -a[1]["p50"])[:5]


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--reps", type=int, default=12); ap.add_argument("--rates", default="1,4,6"); ap.add_argument("--cores", default="2,4")
    ap.add_argument("--json"); ap.add_argument("--md"); ap.add_argument("--port", type=int, default=8979); a = ap.parse_args()
    rates = [int(x) for x in a.rates.split(",")]; cores = [int(x) for x in a.cores.split(",")]
    r = Run(port=a.port, wd="/tmp/wd_macchine"); res = {}
    try:
        with sync_playwright() as p:
            for rate in rates:
                for c in cores:
                    print(f"== {rate}x / {c} cores", flush=True); res[f"{rate}_{c}"] = one_config(p, r.port, rate, c, a.reps)
                    print(json.dumps(res[f"{rate}_{c}"])[:600], flush=True)
    finally:
        r.close()
    sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True).stdout.strip()
    meta = {"date": time.strftime("%Y-%m-%d"), "sha": sha, "cpu": os.cpu_count(), "reps": a.reps, "rates": rates, "cores": cores}
    if a.json: Path(a.json).write_text(json.dumps({"meta": meta, "res": res}, indent=1))
    L = table(res, rates, cores); print("\n".join(L))
    for k, d in worst(res, rates[-1], cores[0]): print(k, d)
    if a.md: Path(a.md).write_text("\n".join(L) + "\n")


if __name__ == "__main__": main()
