"""C29 diagnosis: what happens between the click on «Carica dati» and the work area (#v-data), and how much memory the tab holds.
Measures, for a small and a large synthetic mzML: the instants of (a) work area visible, (b) first chromatogram drawn, (c) map drawn (opened right after),
the calls that run between the click and (a) (functions wrapped with start/end, api calls, long tasks, nodes and canvases created), and the memory of the tab
at four instants (page opened, engine ready, area prepared in advance by a throw-away prototype, after the small file).
It waits for STATES, never for clocks, and asserts no time: it prints a report (MZLAB_E2E_JSON=<file> also writes it as JSON).
"""
import json, os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

# functions of the click -> work area path that are wrapped to log (start, end) relative to the click
WRAP = ["loading", "exploreList", "bootSession", "bootSession0", "paintFiles", "renderFileList", "nbLoad", "restoreUi", "ensureLayout", "ensureBlocks",
        "defaultLayoutTab", "addPanel", "toolbar", "applyView", "relayout", "fitHost", "uiSave", "ctl", "draw", "setup", "rawSeries", "apexSpectrum", "methodWarn"]
INSTR = """
() => {
  window.__ev = []; window.__t0 = null; window.__tA = null; window.__lt = [];
  const rel = () => performance.now() - window.__t0;
  document.querySelector('#opbtn').addEventListener('click', () => { window.__t0 = performance.now(); window.__ev.length = 0; }, true);
  let depth = 0;
  for (const n of %s) {
    const f = window[n]; if (typeof f !== 'function' || f.__m) continue;
    const w = function (...a) {
      if (window.__t0 == null || window.__tA != null && n !== 'draw') return f.apply(this, a);
      const s = rel(), d = depth++, t = a && a[0] && a[0].type ? ' ' + a[0].type : '';
      const end = () => { depth--; window.__ev.push({ n: n + t, s, e: rel(), d }); };
      let r; try { r = f.apply(this, a); } catch (e) { end(); throw e; }
      if (r && typeof r.then === 'function') return r.then(v => { end(); return v; }, e => { end(); throw e; });
      end(); return r;
    };
    w.__m = 1; window[n] = w;
  }
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) window.__lt.push({ s: e.startTime - window.__t0, d: e.duration }); }).observe({ entryTypes: ['longtask'] }); } catch (_) {}
  const v = document.querySelector('#v-data');
  const tick = () => {
    if (window.__t0 != null && window.__tA == null && !v.hidden && v.getBoundingClientRect().height > 0) {
      window.__tA = rel();
      window.__nodesA = document.querySelectorAll('#v-data *').length; window.__canvasA = document.querySelectorAll('#v-data canvas').length;
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}
""" % json.dumps(WRAP)


def big_file(path, scans=6000, noise=600):
    """A Full Scan with many scans (about 40 MB of text): same ions as the lab-like series, much more noise."""
    rng = np.random.default_rng(7)
    rts = np.linspace(0.5, 20.0, scans)
    specs, tic = [], []
    for i, rt in enumerate(rts):
        mz = np.sort(np.concatenate([rng.uniform(120, 480, noise), 364.37 + rng.normal(0, .03, 1)]))
        it = rng.lognormal(np.log(4e3), .9, mz.size) * (1 + 50 * np.exp(-0.5 * ((rt - 14.3) / .06) ** 2) * (np.abs(mz - 364.37) < .1))
        specs.append(ds._spec(i, rt, mz, it)); tic.append(it.sum())
    ds._write(path, specs, [ds._chrom(0, "TIC", rts, tic)])


def pss(pid):
    try:
        for l in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
            if l.startswith("Pss:"): return int(l.split()[1]) / 1024
    except Exception: pass
    return 0.0


def procs(root_pid):
    """All the descendants of the browser process: [(pid, kind)] with kind renderer / gpu / utility / browser."""
    out, kids = [], {}
    for d in Path("/proc").iterdir():
        if not d.name.isdigit(): continue
        try:
            st = (d / "stat").read_text(); ppid = int(st.rsplit(")", 1)[1].split()[1])
            kids.setdefault(ppid, []).append(int(d.name))
        except Exception: pass
    todo = [root_pid]
    while todo:
        p = todo.pop()
        try: cmd = Path(f"/proc/{p}/cmdline").read_bytes().decode(errors="ignore")
        except Exception: continue
        k = "renderer" if "--type=renderer" in cmd else "gpu" if "--type=gpu" in cmd else "utility" if "--type=" in cmd else "browser"
        out.append((p, k)); todo += kids.get(p, [])
    return out


def memory(pg, b, cdp):
    """Memory of the tab: JS heap of the page after a forced GC (CDP) and the PSS of the renderer processes (page + workers), in MB."""
    cdp.send("HeapProfiler.collectGarbage")
    m = {x["name"]: x["value"] for x in cdp.send("Performance.getMetrics")["metrics"]}
    pr = procs(browser_pid())
    ren = sum(pss(p) for p, k in pr if k == "renderer")
    return {"js_heap_mb": round(m["JSHeapUsedSize"] / 1048576, 1), "dom_nodes": int(m["Nodes"]), "listeners": int(m["JSEventListeners"]),
            "renderer_pss_mb": round(ren, 0), "gpu_pss_mb": round(sum(pss(p) for p, k in pr if k == "gpu"), 0), "n_renderers": sum(1 for _, k in pr if k == "renderer")}


def browser_pid():
    """The main process of the Chromium started by Playwright (the one without --type=)."""
    for d in Path("/proc").iterdir():
        if d.name.isdigit():
            try: c = (d / "cmdline").read_bytes().decode(errors="ignore")
            except Exception: continue
            if "chrom" in c.split("\0")[0] and "--type=" not in c and "--remote-debugging-pipe" in c: return int(d.name)
    return 0


def prototype(pg):
    """Throw-away prototype of an area prepared in advance: the work area unhidden off-screen at its final width, three panel canvases at the final size
    with a 2d context and every font the charts use drawn once. Nothing of it is kept in the product."""
    return pg.evaluate("""() => {
      const v = document.querySelector('#v-data'), had = v.hidden;
      v.hidden = false; v.style.cssText += ';position:absolute;left:-99999px;top:0;width:' + (innerWidth - 28) + 'px';
      const d = window.devicePixelRatio || 1, made = [];
      for (let i = 0; i < 3; i++) {
        const c = document.createElement('canvas'); c.width = Math.round((innerWidth - 28) * d); c.height = Math.round(300 * d);
        c.style.cssText = 'width:' + (innerWidth - 28) + 'px;height:300px'; v.appendChild(c);
        const g = c.getContext('2d'); g.font = '12px sans-serif'; g.fillText('m/z 364.4  RT 14.3 min  1.2e6', 10, 10); g.font = 'bold 13px sans-serif'; g.fillText('TIC', 10, 30);
        g.fillRect(0, 0, 4, 4); made.push(c);
      }
      window.__proto = { v, made, had };
      return { canvases: made.length, w: innerWidth - 28 };
    }""")


def prototype_undo(pg):
    pg.evaluate("""() => { const p = window.__proto; p.made.forEach(c => c.remove()); p.v.style.position = p.v.style.left = p.v.style.top = p.v.style.width = ''; p.v.hidden = p.had; }""")


def self_time(prof, top=10):
    """Self time (ms) per function of a CPU profile, the biggest first."""
    nodes = {n["id"]: n for n in prof["nodes"]}; acc = {}
    for nid, dt in zip(prof["samples"], prof["timeDeltas"]):
        cf = nodes[nid]["callFrame"]; k = f"{cf['functionName'] or '(anon)'} {cf['url'].rsplit('/', 1)[-1]}:{cf['lineNumber'] + 1}"
        acc[k] = acc.get(k, 0) + dt / 1000
    return [(k, round(v, 1)) for k, v in sorted(acc.items(), key=lambda kv: -kv[1])[:top] if k not in ("(idle) :1",)]


def scenario(r, p, name, f):
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": 1400, "height": 900}, reduced_motion="reduce", locale="it-IT")
    ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg = ctx.new_page(); cdp = ctx.new_cdp_session(pg); cdp.send("Performance.enable")
    res = {"file": name, "size_mb": round(os.path.getsize(f) / 1048576, 1), "memory": {}}
    pg.goto(f"http://127.0.0.1:{r.port}/?perf=1")
    pg.wait_for_function("document.readyState === 'complete'")
    res["memory"]["1_page_opened"] = memory(pg, b, cdp)
    attendi(pg, "window.qqReady === true"); ready(pg)
    res["start_ms"] = pg.evaluate("(() => { const m = {}; for (const x of PERF.rows) if (x.kind === 'start') m[x.what] = Math.round(x.ms); return m; })()")
    res["memory"]["2_engine_ready"] = memory(pg, b, cdp)
    res["proto"] = prototype(pg); res["memory"]["3_area_in_advance"] = memory(pg, b, cdp); prototype_undo(pg)
    stage(pg, [f]); ready(pg)
    pg.evaluate(INSTR)
    pg.evaluate("window.__perf0 = PERF.rows.length")
    cdp.send("Profiler.enable"); cdp.send("Profiler.setSamplingInterval", {"interval": 200}); cdp.send("Profiler.start")
    pg.click("#opbtn")
    attendi(pg, "window.__tA != null")
    prof = cdp.send("Profiler.stop")["profile"]; ready(pg)
    res["cpu_top"] = self_time(prof)
    res["t_area_ms"] = round(pg.evaluate("window.__tA"))
    res["nodes_at_area"] = pg.evaluate("window.__nodesA"); res["canvases_at_area"] = pg.evaluate("window.__canvasA")
    rows = pg.evaluate("PERF.rows.slice(window.__perf0).map(x => ({kind: x.kind, what: x.what, ms: x.ms, t: x.t}))")
    t0 = pg.evaluate("window.__t0")
    first = lambda pred: next((round(x["t"] - t0) for x in rows if pred(x)), None)
    res["t_first_chrom_ms"] = first(lambda x: x["kind"] == "disegno" and x["what"] == "draw chrom")
    res["t_first_spec_ms"] = first(lambda x: x["kind"] == "disegno" and x["what"] == "draw spec")
    res["api"] = [{"what": x["what"][:40], "end": round(x["t"] - t0), "ms": round(x["ms"])} for x in rows if x["kind"] == "api"]
    res["calls"] = sorted(pg.evaluate("window.__ev"), key=lambda e: e["s"])
    res["longtasks"] = [{"s": round(x["s"]), "d": round(x["d"])} for x in pg.evaluate("window.__lt")]
    res["memory"]["4_after_file"] = memory(pg, b, cdp)
    # (c) the map, opened from the toolbar once the file is in
    t1 = pg.evaluate("performance.now()")
    pg.evaluate("document.querySelector('#np-map').click()")
    attendi(pg, "PERF.rows.some(x => x.kind === 'disegno' && x.what === 'draw map')"); ready(pg)
    mrow = pg.evaluate("PERF.rows.filter(x => x.kind === 'disegno' && x.what === 'draw map').map(x => ({t: x.t, ms: x.ms}))")[0]
    res["map_open_to_drawn_ms"] = round(mrow["t"] - t1); res["map_first_draw_ms"] = round(mrow["ms"])
    res["t_map_since_click_ms"] = round(mrow["t"] - t0)
    res["lr_miss"] = pg.evaluate("(window.MZLAB_LR_MISS || []).length")
    b.close()
    return res


def main():
    r = Run(port=8897, wd="/tmp/wd97")
    out = []
    tmp = Path(tempfile.mkdtemp(prefix="c29_"))
    try:
        small, big = tmp / "B_FullMass-t0.mzML", tmp / "B_FullMass-t15.mzML"
        ds.full_scan(small, 0, np.random.default_rng(1))
        big_file(big)
        with sync_playwright() as p:
            for name, f in (("small", small), ("large", big)):
                out.append(scenario(r, p, name, f))
    finally:
        r.close()
        import shutil; shutil.rmtree(tmp, ignore_errors=True)
    for o in out:
        assert o["t_area_ms"] > 0 and o["t_first_chrom_ms"] is not None, "the work area or the first chromatogram never appeared"
        assert o["lr_miss"] == 0, "the engine did not serve a route"
    dst = get_env("E2E_JSON")
    if dst: Path(dst).write_text(json.dumps(out, indent=1))
    for o in out:
        print(f"\n== {o['file']} ({o['size_mb']} MB)  area {o['t_area_ms']} ms  first chrom {o['t_first_chrom_ms']} ms  first spec {o['t_first_spec_ms']} ms  map opened -> drawn {o['map_open_to_drawn_ms']} ms (draw {o['map_first_draw_ms']} ms)")
        print("   nodes in #v-data at (a):", o["nodes_at_area"], "canvases:", o["canvases_at_area"], " start marks:", o["start_ms"])
        print("   cpu top (self ms):", o["cpu_top"]); print("   api:", o["api"]); print("   longtasks:", o["longtasks"])
        for e in o["calls"]: print(f"   {'  ' * e['d']}{e['n']:<22} {e['s']:7.1f} -> {e['e']:7.1f}  ({e['e'] - e['s']:.1f} ms)")
        for k, v in o["memory"].items(): print("   mem", k, v)
    print("DONE")


main()
