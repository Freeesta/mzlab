"""Measures what moves and what is fetched while files load (study docs/studi/CARICAMENTO_STABILITA.md): serves a built site, opens it in headless Chromium and
records, from the first frame, the box of the main elements at every frame, the layout-shift entries with their sources, every request (time, size, who asked)
and phase marks (open, engine ready, drag-over, drop, each side-list row, batch end, first plot, sample table and back, removal, «+» of the side list, reload with the saved session).

    python3 tests_e2e/misura_caricamento.py --site site --files a.mzML b.mzML [--width 1400] [--dark] [--cold] [--out run.json]

`--cold` keeps the first-visit reload of the cross-origin isolation (browser.js isolateOnce); without it the page is measured as on the second visit.
Prints a summary (shifts per element and phase, requests per phase); the JSON keeps everything. Works on origin/main (clicks «Load data» when it is there) and on D3."""
import argparse, json, subprocess, sys, time, urllib.request
from pathlib import Path
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tests_e2e"))
from lib import free_port      # noqa: E402

WATCH = ["#nav", "#stbar", "#avvio-fase", "#dfiles", "#sb-pane", "#flst", "#fl-up", "#samptab", "#v-workspace", "#tools", "#opening", "#wdata", "#unopen", "#up",
         "#start", "#drop", "#demo", "#wtable", "#flist", "#hrmix", "#opbtn", "#dpanels", "#dpanels .pnl", "#dpanels .pnl canvas", "#loading", "#err", "#fl-note"]
PROBE = """(() => { if (window.__m) return; const W = %s, T0 = performance.timeOrigin; const now = () => Math.round(T0 + performance.now());
  window.__m = { boxes: {}, ls: [], rows: [], marks: [], state: [], last: now() }; try { performance.setResourceTimingBufferSize(20000); } catch (_) {}
  const box = s => { const e = document.querySelector(s); if (!e) return null; const cs = getComputedStyle(e); if (e.hidden || cs.display === 'none') return 'hidden';
    const b = e.getBoundingClientRect(); if (!b.width && !b.height) return 'empty'; return [Math.round(b.left), Math.round(b.top + scrollY), Math.round(b.width), Math.round(b.height), cs.visibility === 'hidden' || +cs.opacity < 0.05 ? 'inv' : ''].join(','); };
  const last = {}, seen = new Set();
  const state = () => [typeof UPBUSY !== 'undefined' ? UPBUSY : '-', typeof OPENNOW !== 'undefined' ? OPENNOW.busy() : '-', (document.getElementById('v-workspace') || {dataset: {}}).dataset.work || '-',
    document.querySelector('#loading') && !document.querySelector('#loading').hidden ? 'veil' : '', 'y' + Math.round(scrollY)].join('|');
  const tick = () => { const t = now(); for (const s of W) { const b = box(s); if (b !== last[s]) { (window.__m.boxes[s] = window.__m.boxes[s] || []).push([t, b]); last[s] = b; window.__m.last = t; } }
    const st = state(); if (st !== last.__st) { window.__m.state.push([t, st]); last.__st = st; window.__m.last = t; }
    document.querySelectorAll('#flst .fl').forEach(r => { const k = (r.classList.contains('up') ? 'up:' : 'file:') + (r.dataset.up || (r.querySelector('.nm') || r).textContent.trim().slice(0, 40)); if (!seen.has(k)) { seen.add(k); window.__m.rows.push([t, k]); } });
    requestAnimationFrame(tick); };
  requestAnimationFrame(tick);
  const who = n => n ? (n.id ? '#' + n.id : (n.className && typeof n.className === 'string' ? n.nodeName.toLowerCase() + '.' + n.className.split(' ').slice(0, 2).join('.') : n.nodeName.toLowerCase())) + (n.parentElement ? ' < ' + (n.parentElement.id ? '#' + n.parentElement.id : n.parentElement.className || n.parentElement.nodeName) : '') : '?';
  new PerformanceObserver(l => { for (const e of l.getEntries()) window.__m.ls.push([Math.round(T0 + e.startTime), +e.value.toFixed(5), e.hadRecentInput, e.sources.map(x => [who(x.node), [x.previousRect.y, x.previousRect.height].map(Math.round), [x.currentRect.y, x.currentRect.height].map(Math.round)])]); }).observe({ type: 'layout-shift', buffered: true });
})()""" % json.dumps(WATCH)
DROP = """async (names) => { const inp = document.getElementById('__probein'); const dt = new DataTransfer(); for (const f of inp.files) dt.items.add(f);
  const tgt = document.querySelector('#start:not([hidden]) #drop') || document.querySelector('#fl-dz') || document.querySelector('#flst');
  for (const k of ['dragenter', 'dragover']) tgt.dispatchEvent(new DragEvent(k, { bubbles: true, cancelable: true, dataTransfer: dt }));
  await new Promise(r => setTimeout(r, 400)); window.__m.marks.push([Math.round(performance.timeOrigin + performance.now()), 'drop']);
  tgt.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: dt })); }"""
IDLE = "(typeof UPBUSY === 'undefined' || UPBUSY === 0) && (typeof OPENNOW === 'undefined' || OPENNOW.busy() === 0) && (typeof PROGRESSO === 'undefined' || !PROGRESSO.busy())"


def quiet(pg, ms):
    """The settle window of a step: no watched box and no state changed for `ms` (a frame loop that never calms down ends after 30 s)."""
    pg.wait_for_function(f"(() => {{ const t = performance.timeOrigin + performance.now(); window.__q = window.__q || t; const ok = t - window.__m.last > {ms} || t - window.__q > 30000; if (ok) window.__q = 0; return ok; }})()", timeout=40000)


def run(a):
    port = free_port()
    srv = subprocess.Popen([sys.executable, str(ROOT / "tests_e2e" / "servi.py"), str(port), str(Path(a.site).resolve())], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(50):
        try: urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=1); break
        except Exception: __import__("threading").Event().wait(0.2)      # the server is starting
    site, reqs, marks, navs = Path(a.site).resolve(), [], [], []
    mark = lambda name: marks.append([round(time.time() * 1000), name])
    def size(url):
        p = url.split(f":{port}/", 1)[-1].split("?")[0].split("#")[0] or "index.html"
        f = site / p
        return f.stat().st_size if f.is_file() else None
    try:
        with sync_playwright() as p:
            b = p.chromium.launch()
            c = b.new_context(viewport={"width": a.width, "height": a.height}, color_scheme="dark" if a.dark else "light")
            c.set_default_timeout(300000)
            c.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
            if not a.cold: c.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');}catch(_){}")
            c.add_init_script(PROBE)
            pg = c.new_page()
            def who(r):
                try: return "page" if r.frame else "worker"
                except Exception: return "worker"
            c.on("request", lambda r: reqs.append([round(time.time() * 1000), r.url.split(f":{port}", 1)[-1], r.resource_type, size(r.url), who(r)]))
            pg.on("framenavigated", lambda fr: navs.append([round(time.time() * 1000), fr.url.split(f":{port}", 1)[-1]]) if fr == pg.main_frame else None)
            mark("goto"); pg.goto(f"http://127.0.0.1:{port}/")
            pg.wait_for_selector("#drop"); mark("drop-visible")
            pg.wait_for_function("window.qqReady === true", timeout=120000); mark("engine-ready")
            quiet(pg, a.settle if not a.cold else 4000); mark("open-settled")
            pg.evaluate("() => { const i = document.createElement('input'); i.type = 'file'; i.multiple = true; i.id = '__probein'; i.style.display = 'none'; document.body.appendChild(i); }")
            pg.set_input_files("#__probein", a.files)
            pg.evaluate(DROP, [Path(f).name for f in a.files]); mark("dragover+drop")
            pg.wait_for_function(f"({IDLE}) && (document.querySelectorAll('#flst .fl').length > 0 || !!document.querySelector('#opbtn:not([disabled])') && !!document.querySelector('#opbtn').offsetParent)", timeout=600000); mark("batch-end")
            if pg.locator("#opbtn").count() and pg.is_visible("#opbtn"):
                quiet(pg, 500); mark("opbtn-click"); pg.click("#opbtn")
            try: pg.wait_for_selector("#dpanels .pnl canvas", timeout=120000); mark("first-plot")
            except Exception: mark("no-plot")
            quiet(pg, a.settle); mark("plot-settled")
            if pg.locator("#samptab").count() and pg.is_visible("#samptab"):
                mark("samptab-click"); pg.click("#samptab"); quiet(pg, a.settle)
                if pg.locator("#flst .fl .nm").count() > 1:
                    mark("file-click"); pg.click("#flst .fl .nm >> nth=1"); pg.wait_for_selector("#dpanels .pnl canvas"); quiet(pg, a.settle)
            if pg.evaluate("E.files.filter(f => !f.gone).length") > 1:
                mark("remove-hidden"); pg.evaluate("removeFile(E.files.find(f => !f.gone && !f.vis) || E.files.filter(f => !f.gone).pop())"); quiet(pg, a.settle)
            mark("remove-shown"); pg.evaluate("removeFile(E.files.find(f => !f.gone && f.vis) || E.files.find(f => !f.gone))"); quiet(pg, a.settle)
            mark("undo"); pg.evaluate("typeof STORIA !== 'undefined' && STORIA.annulla ? STORIA.annulla() : null"); quiet(pg, a.settle)
            if pg.locator("#addf").count() and pg.is_visible("#addf"):
                mark("addf-click"); pg.click("#addf"); quiet(pg, a.settle)
                if pg.is_visible("#backwork"): mark("backwork-click"); pg.click("#backwork"); quiet(pg, a.settle)
            before = pg.evaluate("window.__m")
            before["res"] = pg.evaluate("performance.getEntriesByType('resource').map(e => [Math.round(performance.timeOrigin + e.startTime), e.name.split('/').slice(3).join('/'), e.initiatorType, e.renderBlockingStatus || '', e.decodedBodySize])")
            mark("reload"); pg.reload(); pg.wait_for_selector("#v-data")
            try: pg.wait_for_selector("#dpanels .pnl canvas", timeout=120000); mark("restored-plot")
            except Exception: mark("no-restored-plot")
            quiet(pg, a.settle); mark("end")
            after = pg.evaluate("window.__m")
            b.close()
    finally:
        srv.terminate()
    return {"args": {k: v for k, v in vars(a).items() if k != "out"}, "marks": marks, "navs": navs, "reqs": reqs, "before": before, "after": after}


def phase(t, marks):
    cur = "?"
    for m in sorted(marks):
        if m[0] <= t: cur = m[1]
    return cur


def summary(d):
    out = []
    for part in ("before", "after"):
        m = d[part]; marks = sorted(d["marks"] + m["marks"])
        out.append(f"== {part} reload")
        by = {}
        for t, v, inp, src in m["ls"]:
            k = (phase(t, marks), ";".join(s[0] for s in src[:2]))
            by[k] = [by.get(k, [0, 0])[0] + v, by.get(k, [0, 0])[1] + 1]
        for (ph, src), (v, n) in sorted(by.items(), key=lambda x: -x[1][0]):
            out.append(f"  CLS {v:.4f} ({n}x) during {ph}: {src}")
        for s, tl in m["boxes"].items():
            ch = [(phase(t, marks), b) for t, b in tl]
            if len(ch) > 1: out.append(f"  {s}: {len(ch) - 1} changes: " + " | ".join(f"{p}->{b}" for p, b in ch[:12]) + (" ..." if len(ch) > 12 else ""))
        out.append("  rows: " + ", ".join(f"{phase(t, marks)}:{k}" for t, k in m["rows"][:30]))
        out.append("  state: " + ", ".join(f"{phase(t, marks)}:{s}" for t, s in m["state"][:30]))
    # the service worker refetches what the page asks (page + worker) and precaches the rest on its own (worker only)
    first = {}
    for t, u, kind, sz, who in d["reqs"]:
        k = (phase(t, d["marks"]), u)
        r = first.setdefault(k, [u, sz or 0, set()]); r[2].add(who)
    by = {}
    for (ph, u), r in first.items(): by.setdefault(ph, []).append(r)
    for ph, rs in by.items():
        pgr = [r for r in rs if "page" in r[2]]; sw = [r for r in rs if r[2] == {"worker"}]
        out.append(f"== during {ph}: page {len(pgr)} req {sum(r[1] for r in pgr) / 1e6:.2f} MB; worker/SW only {len(sw)} req {sum(r[1] for r in sw) / 1e6:.2f} MB")
        out.append("   page: " + ", ".join(f"{r[0].split('?')[0][-38:]}({r[1] // 1000}k)" for r in pgr[:60]))
    out.append("navigations: " + ", ".join(f"{round(t - d['marks'][0][0])}ms {u}" for t, u in d["navs"]))
    out.append("marks: " + ", ".join(f"{n}@{round(t - d['marks'][0][0])}" for t, n in d["marks"]))
    return "\n".join(out)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--site", default=str(ROOT / "site")); ap.add_argument("--files", nargs="+", required=True)
    ap.add_argument("--width", type=int, default=1400); ap.add_argument("--height", type=int, default=900); ap.add_argument("--dark", action="store_true")
    ap.add_argument("--cold", action="store_true"); ap.add_argument("--settle", type=int, default=1500); ap.add_argument("--out")
    a = ap.parse_args()
    d = run(a)
    if a.out: Path(a.out).write_text(json.dumps(d), encoding="utf-8")
    print(summary(d))
