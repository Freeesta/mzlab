"""C39 phase 1, measurement only: where the memory of the tab goes, at five instants (page opened / engine ready / demo Full Scan files open / after the map /
after Tabui in the administrator mode). For the page and for every worker: JS heap, WebAssembly linear memory of every instance, buffer bytes posted by copy and
by transfer; for the page also the canvases (w x h x 4 B, hidden ones and the ones not in the DOM too) and the size of the JS caches (CACHE, SC, ...); for the
browser the PSS of the renderer and GPU processes read from /proc. It asserts no threshold: it prints a table sorted by MB with a proposal for each big item
(MZLAB_E2E_JSON=<file> also writes it as JSON). A dedicated worker lives in the process of its creator, so its PSS cannot be split from the page's: the per-worker
figures are heap + WebAssembly memory. The hook that sees the workers is injected before any script (WebAssembly, Worker, Blob and postMessage are wrapped).
"""
import json, os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

# Runs in every realm (page, dedicated workers) before any script of the app. Idempotent. Wrapped with Proxy so `instanceof` keeps working.
HOOK = r"""
(function __mzhook() {
  const HOOKSRC = '(' + __mzhook.toString() + ')()';
  const g = self; if (g.__mz) return;
  const mz = g.__mz = { mems: [], cp: { n: 0, b: 0 }, mv: { n: 0, b: 0 }, cv: [], blobs: new WeakMap(), workers: [] };
  const W = g.WebAssembly;
  if (W) {
    const M = W.Memory, seen = new Set();
    const rec = o => { try { for (const v of Object.values((o && o.exports) || {})) if (v instanceof M && !seen.has(v)) { seen.add(v); mz.mems.push(v); } } catch (_) {} };
    const oi = W.instantiate.bind(W), os = W.instantiateStreaming && W.instantiateStreaming.bind(W);
    W.instantiate = async (...a) => { const r = await oi(...a); rec(r.instance || r); return r; };
    if (os) W.instantiateStreaming = async (...a) => { const r = await os(...a); rec(r.instance); return r; };
    W.Instance = new Proxy(W.Instance, { construct(t, a, nt) { const r = Reflect.construct(t, a, nt); rec(r); return r; } });
    W.Memory = new Proxy(M, { construct(t, a, nt) { const r = Reflect.construct(t, a, nt); if (!seen.has(r)) { seen.add(r); mz.mems.push(r); } return r; } });
  }
  const bytes = (v, d, s) => {
    if (!v || typeof v !== 'object' || s.has(v)) return 0; s.add(v);
    if (v instanceof ArrayBuffer) return v.byteLength;
    if (ArrayBuffer.isView(v)) return v.byteLength;
    if (d > 3) return 0; let b = 0;
    for (const k in v) { try { b += bytes(v[k], d + 1, s); } catch (_) {} }
    return b;
  };
  const note = (m, t) => {
    const b = bytes(m, 0, new Set()); if (!b) return;
    const tr = Array.isArray(t) ? t : (t && t.transfer) || [], tb = tr.reduce((s, x) => s + (x && x.byteLength || 0), 0);
    if (tb) { mz.mv.n++; mz.mv.b += tb; }
    if (b > tb) { mz.cp.n++; mz.cp.b += b - tb; }
  };
  if (typeof WorkerGlobalScope !== 'undefined' && g instanceof WorkerGlobalScope) {
    const pm = g.postMessage.bind(g); g.postMessage = (m, t) => { note(m, t); return pm(m, t); };
  }
  {
    if (g.Worker) {
      const pp = Worker.prototype.postMessage; Worker.prototype.postMessage = function (m, t) { note(m, t); return pp.call(this, m, t); };
      g.Worker = new Proxy(g.Worker, { construct(t, a, nt) {
        let [u, o] = a; o = Object.assign({}, o); let abs; try { abs = new URL(String(u), location.href).href; } catch (_) { return Reflect.construct(t, a, nt); }
        let label = abs.split('/').pop();
        if (abs.startsWith('blob:')) { label = 'blob:' + (mz.blobs.get(g.__mzUrl && g.__mzUrl[abs]) || '?'); if (!o.name) o.name = label; return Reflect.construct(t, [u, o], nt); }
        o.name = o.name || label;
        const boot = (o.type === 'module' ? 'import ' + JSON.stringify(abs) + ';' : 'importScripts(' + JSON.stringify(abs) + ');');
        const w = Reflect.construct(t, [URL.createObjectURL(new Blob([HOOKSRC + ';\n' + boot], { type: 'text/javascript' })), o], nt);
        mz.workers.push({ label, abs }); return w;
      } });
    }
    if (typeof document !== 'undefined' && g.HTMLCanvasElement) {
      const gc = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function (...a) { if (!this.__mzReg) { this.__mzReg = 1; mz.cv.push(new WeakRef(this)); } return gc.apply(this, a); };
    }
  }
  // workers started from a Blob of the app (Tabui): the hook goes in front of the source, the first line names it
  if (g.Blob && g.URL) {
    g.__mzUrl = {};
    const B = g.Blob;
    g.Blob = new Proxy(B, { construct(t, a, nt) {
      let [p, o] = a;
      if (o && /javascript/.test(o.type || '') && p && p.length && p.every(x => typeof x === 'string') && !String(p[0]).startsWith('/*mzhook*/')) {
        const head = String(p[0]).slice(0, 600).split('\n').find(l => l.trim() && !/ts-check/.test(l)) || '';
        p = ['/*mzhook*/' + HOOKSRC + ';\n', ...p];
        const r = Reflect.construct(t, [p, o], nt); mz.blobs.set(r, head.trim().slice(0, 60)); return r;
      }
      return Reflect.construct(t, a, nt);
    } });
    const cu = URL.createObjectURL.bind(URL); URL.createObjectURL = b => { const u = cu(b); if (b && mz.blobs.has(b)) g.__mzUrl[u] = b; return u; };
  }
  mz.report = () => {
    const pm = (typeof performance !== 'undefined' && performance.memory) || null;
    return { name: g.name || '', heap_mb: pm ? pm.usedJSHeapSize / 1048576 : null, heap_total_mb: pm ? pm.totalJSHeapSize / 1048576 : null,
             wasm_mb: mz.mems.map(m => m.buffer.byteLength / 1048576), copied_mb: mz.cp.b / 1048576, copies: mz.cp.n, moved_mb: mz.mv.b / 1048576, moves: mz.mv.n };
  };
})();
"""
INIT = HOOK.strip()

# size (bytes) of the data a cache keeps: typed arrays by byteLength (once), strings 1 B per char, numbers 8 B, plus 8 B per slot
SIZER = r"""
(async () => {
  const sz = (v, seen, st) => {
    if (v == null) return 0; const t = typeof v;
    if (t === 'number') return 8; if (t === 'string') return v.length; if (t === 'boolean') return 4; if (t !== 'object') return 0;
    if (seen.has(v) || st.n++ > 4e6) return 0; seen.add(v);
    if (v instanceof ArrayBuffer) return v.byteLength;
    if (ArrayBuffer.isView(v)) { if (seen.has(v.buffer)) return 0; seen.add(v.buffer); return v.buffer.byteLength; }
    if ((typeof Node !== 'undefined' && v instanceof Node) || v === window) return 0;
    let b = 16;
    if (v instanceof Map) { for (const [k, x] of v) b += sz(k, seen, st) + sz(x, seen, st); }
    else if (v instanceof Set) { for (const x of v) b += sz(x, seen, st); }
    else if (Array.isArray(v)) { for (let i = 0; i < v.length; i++) b += 8 + sz(v[i], seen, st); }
    else { for (const k of Object.keys(v)) b += 8 + sz(v[k], seen, st); }
    return b;
  };
  const names = %s;
  const lex = { %s };
  const out = [];
  for (const n of names) {
    let v = lex[n]; if (v === undefined) continue;
    if (n === 'CACHE' && v instanceof Map) { const vals = await Promise.allSettled([...v.values()]); out.push({ name: 'CACHE', entries: v.size, mb: sz(vals.map(x => x.value), new Set(), { n: 0 }) / 1048576 }); continue; }
    if (n === 'SC') { out.push({ name: 'SC.m', entries: v.m.size, mb: sz(v.m, new Set(), { n: 0 }) / 1048576 }); continue; }
    out.push({ name: n, entries: (v && (v.size || v.length || Object.keys(v).length)) || 0, mb: sz(v, new Set(), { n: 0 }) / 1048576 });
  }
  // every other global the scripts added to window
  const f = document.createElement('iframe'); document.body.appendChild(f); const base = new Set(Object.getOwnPropertyNames(f.contentWindow)); f.remove();
  for (const k of Object.getOwnPropertyNames(window)) {
    if (base.has(k) || names.includes(k) || k.startsWith('__') || k === 'webkitStorageInfo') continue;
    let v; try { v = window[k]; } catch (_) { continue; } if (!v || typeof v !== 'object') continue;
    const mb = sz(v, new Set(), { n: 0 }) / 1048576; if (mb >= 0.25) out.push({ name: 'window.' + k, entries: (v.size || v.length || 0), mb });
  }
  const cv = [...document.querySelectorAll('canvas')], inDom = new Set(cv);
  const hidden = c => !c.getClientRects().length;
  const mk = c => ({ id: c.id || c.className || c.parentElement && (c.parentElement.id || c.parentElement.className) || '?', w: c.width, h: c.height, mb: c.width * c.height * 4 / 1048576, hidden: hidden(c), dom: inDom.has(c) });
  const all = cv.map(mk);
  for (const r of (window.__mz ? window.__mz.cv : [])) { const c = r.deref(); if (c && !inDom.has(c)) { inDom.add(c); all.push({ ...mk(c), dom: false, hidden: true }); } }
  const st = navigator.storage && navigator.storage.estimate ? await navigator.storage.estimate().catch(() => null) : null;
  return { caches: out.sort((a, b) => b.mb - a.mb).slice(0, 14), canvases: all.sort((a, b) => b.mb - a.mb), storage_mb: st ? st.usage / 1048576 : null };
})()
"""
# globals of the classic scripts are lexical (not on window) and eval is forbidden by the CSP: each one is named here with a typeof guard
LEX = ["CACHE", "SC", "E", "S", "NB", "PERF", "STORIA"]
SIZER = SIZER % (json.dumps(LEX), ", ".join(f"{n}: typeof {n} !== 'undefined' ? {n} : undefined" for n in LEX))


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


def browser_pid():
    """The main process of the Chromium started by Playwright (the one without --type=)."""
    for d in Path("/proc").iterdir():
        if d.name.isdigit():
            try: c = (d / "cmdline").read_bytes().decode(errors="ignore")
            except Exception: continue
            if "chrom" in c.split("\0")[0] and "--type=" not in c and "--remote-debugging-pipe" in c: return int(d.name)
    return 0


def measure(pg, ctx, cdp):
    """Everything measured at one instant, after a forced GC of the page (CDP) and of every worker (a worker has no CDP session here: its own heap is read as is)."""
    cdp.send("HeapProfiler.collectGarbage")
    m = {x["name"]: x["value"] for x in cdp.send("Performance.getMetrics")["metrics"]}
    pr = procs(browser_pid())
    page = pg.evaluate("() => window.__mz ? window.__mz.report() : null")
    page_x = pg.evaluate(SIZER)
    workers = []
    for w in pg.workers:
        try:
            rep_ = w.evaluate("() => self.__mz ? self.__mz.report() : null")
            workers.append({"url": w.url[:60], **rep_} if rep_ else {"url": w.url[:60], "error": "no hook (a worker started by another worker)"})
        except Exception as e: workers.append({"url": w.url[:60], "error": str(e)[:80]})
    sw = []
    for s in ctx.service_workers:
        try: sw.append({"url": s.url.rsplit("/", 1)[-1], "heap_mb": s.evaluate("() => performance.memory ? performance.memory.usedJSHeapSize / 1048576 : null")})
        except Exception as e: sw.append({"url": s.url.rsplit("/", 1)[-1], "error": str(e)[:80]})
    return {"page": {"heap_mb": m["JSHeapUsedSize"] / 1048576, "dom_nodes": int(m["Nodes"]), "wasm_mb": (page or {}).get("wasm_mb", []),
                     "copied_mb": (page or {}).get("copied_mb", 0), "moved_mb": (page or {}).get("moved_mb", 0)},
            "workers": workers, "service_workers": sw, "caches": page_x["caches"], "canvases": page_x["canvases"], "storage_mb": page_x["storage_mb"],
            "renderer_pss_mb": sorted((round(pss(p), 1) for p, k in pr if k == "renderer"), reverse=True),
            "gpu_pss_mb": round(sum(pss(p) for p, k in pr if k == "gpu"), 1), "browser_pss_mb": round(sum(pss(p) for p, k in pr if k == "browser"), 1)}


# a proposal per kind of item; none of them is a fixed cap in MB: a limit would take resources from an upload that needs them
PROPOSAL = {
    "wasm": "windowed data (OPFS, read by block) and compact f32 formats; the memory never shrinks, so a worker idle after its phase is closed and started again on demand",
    "worker_heap": "no copy of the data in JS: arrays are transferred and given back, caches of parsed results kept by share of the free memory, not by count",
    "page_heap": "caches (CACHE, SC) bounded by bytes with the oldest entry out first, and by the share of memory still free, never by a fixed MB",
    "canvas_hidden": "a hidden canvas keeps 1x1 until it is shown (no full resolution while hidden); the backing store of a visible one follows its box x dpr",
    "canvas": "backing store limited to the visible box x dpr; zoom redraws instead of enlarging the bitmap",
    "cache": "limit by bytes (LRU), computed from the memory still free; entries rebuilt from the engine on demand",
    "copies": "post with the transfer list (and recycle) instead of the structured clone: every copy is a second live array until the next GC",
    "browser": "Chromium's own process (UI, network, storage): not the app's",
    "renderer": "sum of the items above plus the engine; closing idle workers and releasing hidden canvases is what lowers it",
    "gpu": "textures follow the canvases: fewer and smaller canvases lower it",
}


def rows_of(snap):
    """The items of one snapshot as [(MB, label, kind, how)] for the final table."""
    out = []
    pg = snap["page"]
    out.append((pg["heap_mb"], "page: JS heap", "page_heap", "CDP Performance.getMetrics after GC"))
    for i, mb in enumerate(snap["renderer_pss_mb"]): out.append((mb, f"renderer process #{i + 1}: PSS", "renderer", "/proc smaps_rollup (page + its dedicated workers, or the service worker)"))
    out.append((snap["gpu_pss_mb"], "gpu process: PSS", "gpu", "/proc smaps_rollup"))
    out.append((snap["browser_pss_mb"], "browser process: PSS", "browser", "/proc smaps_rollup"))
    for i, mb in enumerate(pg["wasm_mb"]): out.append((mb, f"page: WebAssembly memory #{i + 1}", "wasm", "Memory.buffer.byteLength"))
    for w in snap["workers"]:
        if w.get("error"): continue
        nm = w.get("name") or w["url"]
        if w.get("heap_mb") is not None: out.append((w["heap_mb"], f"worker {nm}: JS heap", "worker_heap", "performance.memory"))
        for i, mb in enumerate(w.get("wasm_mb", [])): out.append((mb, f"worker {nm}: WebAssembly memory #{i + 1}", "wasm", "Memory.buffer.byteLength"))
        if w.get("copied_mb", 0) >= 1: out.append((w["copied_mb"], f"worker {nm}: buffers posted by copy ({w['copies']} messages)", "copies", "sum over the run, not live"))
    for s in snap["service_workers"]:
        if s.get("heap_mb"): out.append((s["heap_mb"], f"service worker {s['url']}: JS heap", "worker_heap", "performance.memory"))
    if pg["copied_mb"] >= 1: out.append((pg["copied_mb"], "page: buffers posted by copy to workers", "copies", "sum over the run, not live"))
    cv = snap["canvases"]
    vis, hid = [c for c in cv if not c["hidden"]], [c for c in cv if c["hidden"]]
    out.append((sum(c["mb"] for c in vis), f"canvases visible ({len(vis)})", "canvas", "w x h x 4 B"))
    out.append((sum(c["mb"] for c in hid), f"canvases hidden or detached ({len(hid)}), " + ", ".join(f"{c['id'][:18]} {c['w']}x{c['h']}" for c in hid[:3]), "canvas_hidden", "w x h x 4 B"))
    for c in snap["caches"]:
        if c["mb"] >= 0.5: out.append((c["mb"], f"JS cache {c['name']} ({c['entries']} entries)", "cache", "size estimate"))
    return [(round(mb, 1), lab, kind, how) for mb, lab, kind, how in out]


def attendi_workers(pg, n0, limit_s=40):
    """Wait, sampling once a second, until a worker is gone or `limit_s` seconds pass (bounded: the baseline never closes one)."""
    import time as _t
    t0 = _t.time()
    while len(pg.workers) >= n0 and _t.time() - t0 < limit_s: pg.wait_for_function("document.readyState === 'complete'"); _t.sleep(1)


def scenario(r, p, files):
    b = p.chromium.launch()
    ctx = b.new_context(viewport={"width": 1400, "height": 900}, reduced_motion="reduce", locale="it-IT")
    ctx.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');localStorage.setItem('mzlab.admin','1');}catch(_){}")
    ctx.add_init_script(INIT) if not os.environ.get("NOHOOK") else None
    pg = ctx.new_page(); cdp = ctx.new_cdp_session(pg); cdp.send("Performance.enable")
    snaps, notes = {}, []
    pg.goto(f"http://127.0.0.1:{r.port}/?perf=1")
    pg.wait_for_function("document.readyState === 'complete'")
    snaps["1_page_opened"] = measure(pg, ctx, cdp)
    attendi(pg, "window.qqReady === true"); ready(pg)
    snaps["2_engine_ready"] = measure(pg, ctx, cdp)
    stage(pg, files); pg.click("#opbtn")
    attendi(pg, "!document.querySelector('#v-data').hidden && PERF.rows.some(x => x.kind === 'disegno' && x.what === 'draw chrom')", 90000); ready(pg)
    snaps["3_demo_files_open"] = measure(pg, ctx, cdp)
    pg.evaluate("document.querySelector('#np-map').click()")
    attendi(pg, "PERF.rows.some(x => x.kind === 'disegno' && x.what === 'draw map')", 60000); ready(pg)
    snaps["4_after_map"] = measure(pg, ctx, cdp)
    try:
        # Tabui takes the files already open in the page
        pg.click('#nav button[data-v="tpmine"]'); pg.wait_for_selector("#tp-go", timeout=60000)
        pg.wait_for_function("document.querySelectorAll('#tp-files tr').length > 1", timeout=20000)
    except Exception as e:
        notes.append("Tabui step incomplete: " + str(e).splitlines()[0][:120])
        notes.append("Tabui state: " + pg.evaluate("(document.querySelector('#tp-load')||{}).textContent + ' | ' + (document.querySelector('#tp-files')||{}).innerText")[:300])
    ready(pg)
    snaps["5_after_tabui"] = measure(pg, ctx, cdp)
    # the Pipeline of Tabui on the files already open: preview, confirm the times proposed from the names, run on the series
    try:
        pg.evaluate("PIPE.open()"); pg.wait_for_selector("#pipe[open]", timeout=30000)
        pg.click("#pipe-go"); pg.wait_for_function("document.querySelector('#pipe-n').textContent.length > 0", timeout=60000)
        pg.wait_for_selector("#pr-all", timeout=20000)
        rows = pg.evaluate("[...document.querySelectorAll('#pr-tab .pr-role')].map(e => e.closest('tr').innerText)")
        for i, txt in enumerate(r_.split("\t")[0] for r_ in rows):          # roles and times by the names of the series (the panel proposes them only when the notebook knows them)
            role = "blank" if "blank" in txt else "dark" if "dark" in txt else "t0" if "-t0" in txt else "sample"
            pg.select_option(f"#pr-r{i}", role)
            tm = {"-t0": "0", "-t5": "5", "-t10": "10"}.get(next((k for k in ("-t10", "-t5", "-t0") if k in txt), ""))
            if tm is not None and not pg.is_disabled(f"#pr-t{i}"): pg.fill(f"#pr-t{i}", tm); pg.press(f"#pr-t{i}", "Enter")
        pg.click("#pr-all")
        pg.wait_for_function("!document.querySelector('#pipe-run-go').disabled", timeout=20000)
        pg.click("#pipe-run-go")
        pg.wait_for_function("document.querySelectorAll('#pv-list .pv-row').length > 0 || document.querySelector('#pipe-tab #ft-t')", timeout=120000)
    except Exception as e:
        notes.append("Pipeline step incomplete: " + str(e).splitlines()[0][:120])
    ready(pg)
    snaps["6_after_pipeline"] = measure(pg, ctx, cdp)
    # idle: the workers of the Pipeline and of Tabui are not needed any more; wait (by state, bounded) until one of them is gone
    n0 = len(pg.workers)
    attendi_workers(pg, n0)
    snaps["7_pipeline_idle"] = measure(pg, ctx, cdp)
    notes.append(f"workers alive: {n0} after the Pipeline, {len(pg.workers)} after the idle wait")
    # back to the Data tab, every panel closed: what the canvases of the closed panels keep
    pg.keyboard.press("Escape"); pg.wait_for_function("!document.querySelector('#pipe[open]')", timeout=10000)
    pg.click('#nav button[data-v="data"]'); ready(pg)
    pg.evaluate("[...E.panels].forEach(p => closePanel(p))"); ready(pg)
    snaps["8_panels_closed"] = measure(pg, ctx, cdp)
    notes.append("lr_miss=%d" % pg.evaluate("(window.MZLAB_LR_MISS || []).length"))
    b.close()
    return snaps, notes


def table(snaps):
    last = list(snaps.values())[-1]
    rows = sorted(rows_of(last), key=lambda x: -x[0])
    print("\n== final table (instant 5), MB, biggest first")
    print(f"{'MB':>8}  {'item':<70} proposal")
    for mb, lab, kind, how in rows:
        if mb < 1: continue
        print(f"{mb:8.1f}  {lab[:70]:<70} {PROPOSAL[kind]}")
    return rows


def main():
    r = Run(port=8898, wd="/tmp/wd98")
    tmp = Path(tempfile.mkdtemp(prefix="c39_"))
    try:
        files = []
        # a series with the controls the Pipeline asks for (blank, dark) and three times: roles and times are proposed from the names
        for i, (nm, t) in enumerate((("blank", 0), ("dark", 0), ("t0", 0), ("t5", 5), ("t10", 10))):
            f = tmp / f"B_FullMass-{nm}.mzML"; ds.full_scan(f, t, np.random.default_rng(i + 1)); files.append(f)
        with sync_playwright() as p:
            snaps, notes = scenario(r, p, files)
    finally:
        r.close()
        import shutil; shutil.rmtree(tmp, ignore_errors=True)
    assert snaps["3_demo_files_open"]["page"] is not None, "the hook did not run in the page"
    print("\n== per instant: renderer PSS (MB, each process) | gpu | page heap | engine and other workers (heap, wasm)")
    for k, s in snaps.items():
        print(f"{k:<20} renderers {s['renderer_pss_mb']} gpu {s['gpu_pss_mb']} browser {s['browser_pss_mb']} | page heap {s['page']['heap_mb']:.1f} wasm {[round(x, 1) for x in s['page']['wasm_mb']]}")
        for w in s["workers"]:
            print(f"{'':<20}   worker {(w.get('name') or w['url'])[:40]:<40} " + (w.get("error") or f"heap {w['heap_mb'] and round(w['heap_mb'], 1)} wasm {[round(x, 1) for x in w['wasm_mb']]} copied {w['copied_mb']:.1f} MB/{w['copies']} msgs moved {w['moved_mb']:.1f} MB/{w['moves']}"))
        for w in s["service_workers"]: print(f"{'':<20}   service worker {w}")
        cv = s["canvases"]; print(f"{'':<20}   canvases {len(cv)} = {sum(c['mb'] for c in cv):.1f} MB, hidden/detached {sum(c['mb'] for c in cv if c['hidden']):.1f} MB; caches " + ", ".join(f"{c['name']} {c['mb']:.1f}" for c in s['caches'][:5]))
    rows = table(snaps)
    for n in notes: print("note:", n)
    dst = get_env("E2E_JSON")
    if dst: Path(dst).write_text(json.dumps({"snapshots": snaps, "rows": rows, "notes": notes}, indent=1))
    print("DONE")


if __name__ == "__main__":
    main()
