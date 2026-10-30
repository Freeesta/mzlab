"""Landing measure: the states of the first screen and of the first minutes, on the built site (reduced motion, 1440x900, synthetic files only).
Prints ONE JSON line with the keys first, drop, first_panel, proposal_race, hr_drop, remove_all, render_cost, w820, return. Only what is true on both the
old and the new landing is asserted (no page error, no horizontal scroll at 820 px, rows arrive, CLS of the first paint); every other number is only printed."""
import json, os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

CLS_MAX = 0.01
TIMES = (0, 15, 30, 45, 60)
INIT = "try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"

# Vitals from the navigation: observers are installed before any script runs.
VITALS = """() => { const V = window.__v = {fcp: null, lcp: null, cls: 0};
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) if (e.name === 'first-contentful-paint') V.fcp = e.startTime; }).observe({type: 'paint', buffered: true}); } catch (_) {}
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) V.lcp = e.startTime; }).observe({type: 'largest-contentful-paint', buffered: true}); } catch (_) {}
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) V.cls += e.value; }).observe({type: 'layout-shift', buffered: true}); } catch (_) {} }"""

# A rAF timeline of the file lists and of the layout shifts since the call; read with TIMELINE_STOP.
TIMELINE = """() => { const T = window.__tl = {t0: performance.now(), frames: [], cls: 0, stop: false}, n = s => document.querySelectorAll(s).length;
  try { new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) T.cls += e.value; }).observe({type: 'layout-shift', buffered: false}); } catch (_) {}
  const tick = () => { const note = document.querySelector('#fl-note');
    T.frames.push({t: Math.round(performance.now() - T.t0), rows: n('#flst .fl:not(.pr)'), prog: n('#flst .fl.pr'), flist: n('#flist tbody tr'), note: note && !note.hidden ? note.textContent.trim().slice(0, 80) : '',
      pnl: n('.pnl'), start: !!document.querySelector('#start:not([hidden])')});
    if (!T.stop) requestAnimationFrame(tick); };
  requestAnimationFrame(tick); }"""
TIMELINE_STOP = "() => { window.__tl.stop = true; return window.__tl; }"

# A drop through a real DataTransfer: the files sit on a temporary <input type=file>, then dragenter/dragover/drop are dispatched on the target.
DROP = """(sel) => { const inp = document.querySelector('#__tmpdrop'), t = document.querySelector(sel), dt = new DataTransfer();
  for (const f of inp.files) dt.items.add(f);
  for (const ty of ['dragenter', 'dragover', 'drop']) t.dispatchEvent(new DragEvent(ty, {bubbles: true, cancelable: true, dataTransfer: dt}));
  inp.remove(); return dt.files.length; }"""


def synth(d):
    d = Path(d); d.mkdir(parents=True, exist_ok=True); rng = np.random.default_rng(3)
    for t in TIMES: ds.full_scan(d / f"B_FullMass-t{t}.mzML", t, rng)
    ds.hr_dda(d / ds.HR_NAMES["exploris"], np.random.default_rng(31), "exploris")
    return [d / f"B_FullMass-t{t}.mzML" for t in TIMES], d / ds.HR_NAMES["exploris"]


def new_page(r, p, width=1440, height=900, url=""):
    pg = r.b.new_page(viewport={"width": width, "height": height}, reduced_motion="reduce", locale="it-IT")
    pg.add_init_script(INIT)
    pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
    pg.on("response", lambda x: r.errs.append(("http%d" % x.status, x.url)) if x.status >= 400 else None)
    pg.add_init_script("(" + VITALS + ")()")
    pg.goto(f"http://127.0.0.1:{r.port}/" + url)
    return pg


def drop_files(pg, files, sel):
    pg.evaluate("() => { const i = document.createElement('input'); i.type = 'file'; i.multiple = true; i.id = '__tmpdrop'; i.style.display = 'none'; document.body.appendChild(i); }")
    lib._real_sif(pg, "#__tmpdrop", [str(f) for f in files])
    return pg.evaluate(DROP, sel)


def visible(pg, sel):
    return pg.evaluate("(s) => { const e = document.querySelector(s); if (!e || e.hidden) return false; const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== 'hidden'; }", sel)


def first_screen(pg):
    pg.wait_for_function("document.readyState === 'complete'")
    frames(pg, 6)
    v = pg.evaluate("() => ({fcp: window.__v.fcp, lcp: window.__v.lcp, cls: window.__v.cls})")
    inside = pg.evaluate("() => { const e = document.querySelector('#drop'); if (!e) return null; const r = e.getBoundingClientRect(); return r.width > 0 && r.top < innerHeight && r.bottom > 0 && r.top >= 0; }")
    stops = []
    for _ in range(40):
        pg.keyboard.press("Tab")
        a = pg.evaluate("() => { const e = document.activeElement; return e && e !== document.body ? (e.id ? '#' + e.id : e.tagName.toLowerCase() + (e.getAttribute('aria-label') ? '[' + e.getAttribute('aria-label').slice(0, 24) + ']' : '')) : ''; }")
        stops.append(a)
        if a in ("#drop", "#cartpick"): break
    hit = stops[-1] in ("#drop", "#cartpick")
    return {**v, "drop_in_first_screen": inside, "tab_stops": len(stops) if hit else None, "tab_path": stops, "tab_reaches_drop": hit}


def first_panel(pg):
    pg.evaluate(TIMELINE)
    pg.wait_for_selector("#flst .fl .nm", timeout=90000)
    pg.evaluate("document.querySelector('#flst .fl .nm').click()"); how = "first_row"
    try: pg.wait_for_selector(".pnl", timeout=90000)
    except Exception: pass
    ready(pg); frames(pg, 4)
    tl = pg.evaluate(TIMELINE_STOP)
    t_pnl = next((f["t"] for f in tl["frames"] if f["pnl"]), None)
    return {"how": how, "cls": round(tl["cls"], 5), "t_first_pnl_ms": t_pnl, "panels": tl["frames"][-1]["pnl"] if tl["frames"] else 0}


def race(pg):
    return pg.evaluate("""() => { const out = [], K = f => typeof CAMPIONI !== 'undefined' && CAMPIONI.kindOf ? CAMPIONI.kindOf(f) : null;
      for (const f of (typeof ST !== 'undefined' ? ST.files : [])) { const e = (typeof E !== 'undefined' ? E.files : []).find(x => !x.gone && (x.file === f.name || x.file.startsWith(f.name + '#')));
        out.push({name: f.name, st_role: f.kind ?? null, st_time: f.time ?? null, open: !!e, e_role: e ? K(e) : null, e_time: e ? (e.time ?? null) : null,
          agree: !e || (K(e) === (f.kind ?? null) && (e.time ?? null) === (f.time ?? null))}); }
      return out; }""")


def remove_all(pg):
    state = """() => { const v = s => { const e = document.querySelector(s); if (!e || e.hidden) return false; const r = e.getBoundingClientRect(); return r.width * r.height > 0; };
      return {sess: typeof S !== 'undefined' ? !!S.sess : null, panels: document.querySelectorAll('#dpanels .pnl').length, files: E.files.filter(f => !f.gone).length,
        landing: v('#start'), table: v('#flist') || v('#sample-table') || v('#cmp-tab'), panels_visible: v('#dpanels .pnl'), data_view: v('#v-data')}; }"""
    pg.evaluate("removeAllFiles()"); frames(pg, 4)
    after = pg.evaluate(state)
    undone = pg.evaluate("() => { try { return !!STORIA.annulla(); } catch (_) { return false; } }")
    frames(pg, 4)
    return {"after_remove": after, "undo_called": undone, "after_undo": pg.evaluate(state)}


def render_cost(pg):
    return pg.evaluate("""() => { const out = {}, realS = ST.files, realE = E.files, box = document.createElement('div'),
      st = n => Array.from({length: n}, (_, i) => ({name: 'S' + i + '.mzML', size: 1e6, use: true, kind: 'full', time: i, hr: false, polarity: 'pos'})),
      ef = n => Array.from({length: n}, (_, i) => ({k: i, file: 'S' + i + '.mzML', label: 'S' + i, kind: 'full', time: i, lv: 1, type: 'sample'}));
      try { for (const n of [50, 300, 1000]) {
        ST.files = st(n); let t = performance.now(); try { renderFiles(); out['renderFiles_' + n] = Math.round(performance.now() - t); } catch (_) { out['renderFiles_' + n] = null; }
        E.files = ef(n); t = performance.now(); try { CAMPIONI.paint(box); out['paint_' + n] = Math.round(performance.now() - t); } catch (_) { out['paint_' + n] = null; } }
      } finally { ST.files = realS; E.files = realE; try { renderFiles(); } catch (_) {} }
      return out; }""")


def w820(r, p, files):
    pg = new_page(r, p, 820, 900, "?telefono=0")
    pg.wait_for_selector("#drop, #dfiles", state="attached")
    def state():
        return pg.evaluate("() => { const v = document.querySelector('#v-data'); return {sb_open: !!v && v.classList.contains('sb-open'), sb_sheet: !!v && v.classList.contains('sb-sheet'),"
                           " scroll_width: document.documentElement.scrollWidth, inner_width: innerWidth, no_hscroll: document.documentElement.scrollWidth <= innerWidth}; }")
    out = {"landing": state()}
    sel = "#drop" if visible(pg, "#drop") else "#dfiles"
    drop_files(pg, files[:2], sel)
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use], #flst .fl:not(.pr)').length >= 2", timeout=60000)
    open_first(pg)
    try: pg.wait_for_selector(".pnl", timeout=90000)
    except Exception: pass
    ready(pg); frames(pg, 4)
    out["after_first_panel"] = state()
    pg.close()
    return out


def return_visit(r, p, files):
    """Open a session, save it, reload: time to the first panel and whether the landing showed in any frame."""
    pg = new_page(r, p)
    sel = "#drop" if visible(pg, "#drop") else "#dfiles"
    drop_files(pg, files[:2], sel)
    pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use], #flst .fl:not(.pr)').length >= 2", timeout=60000)
    open_first(pg)
    pg.wait_for_selector(".pnl", timeout=90000); ready(pg); saved(pg)
    pg.add_init_script("""window.__ret = {t0: null, start_seen: false, t_pnl: null, frames: 0};
      (function tick() { const R = window.__ret; if (R.t0 === null) R.t0 = performance.now(); R.frames++;
        const s = document.querySelector('#start'); if (s && !s.hidden) { const b = s.getBoundingClientRect(); if (b.width * b.height > 0) R.start_seen = true; }
        if (R.t_pnl === null && document.querySelector('.pnl')) R.t_pnl = Math.round(performance.now() - R.t0);
        if (R.t_pnl === null) requestAnimationFrame(tick); })();""")
    pg.reload()
    pg.wait_for_selector(".pnl", timeout=90000); ready(pg)
    res = pg.evaluate("() => window.__ret")
    pg.close()
    return res


if __name__ == "__main__":
    r = Run(port=8830 + 190, wd="/tmp/wd_landing"); res = {}; steps = []
    def step(name, fn):
        try: fn(); steps.append((name, "ok"))
        except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
    try:
        lr, hr = synth(tempfile.mkdtemp())
        with sync_playwright() as p:
            r.b = p.chromium.launch()
            pg = new_page(r, p)
            res["first"] = first_screen(pg)
            # the drop: on #drop if it is on screen, else on the side panel
            sel = "#drop" if visible(pg, "#drop") else "#dfiles"
            pg.evaluate(TIMELINE)
            n = drop_files(pg, lr, sel)
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use], #flst .fl:not(.pr)').length >= %d" % len(lr), timeout=90000)
            frames(pg, 6)
            tl = pg.evaluate(TIMELINE_STOP)
            fr = tl["frames"]
            t_first = next((f["t"] for f in fr if f["rows"]), None); t_last = next((f["t"] for f in fr if f["rows"] >= len(lr)), None)
            t_flist = next((f["t"] for f in fr if f["flist"] >= len(lr)), None)
            res["drop"] = {"target": sel, "dropped": n, "t_first_row_ms": t_first, "t_last_row_ms": t_last, "t_last_flist_row_ms": t_flist, "cls": round(tl["cls"], 5),
                           "max_prog": max([f["prog"] for f in fr] or [0]), "notes": sorted({f["note"] for f in fr if f["note"]}),
                           "timeline": [f for i, f in enumerate(fr) if i == 0 or (f["rows"], f["prog"], f["flist"], f["note"]) != (fr[i - 1]["rows"], fr[i - 1]["prog"], fr[i - 1]["flist"], fr[i - 1]["note"])]}
            res["proposal_race"] = race(pg)
            res["first_panel"] = first_panel(pg)
            ready(pg)
            # the HR file dropped on the side panel of the open LR session
            pg.evaluate("() => { window.__hr0 = document.querySelectorAll('#flst .fl:not(.pr)').length; }")
            drop_files(pg, [hr], "#dfiles")
            try: pg.wait_for_function("(() => { const n = document.querySelector('#fl-note'); return n && !n.hidden && /\\S/.test(n.textContent) && !/\\.\\.\\.|…/.test(n.textContent); })()", timeout=30000)
            except Exception: pass
            ready(pg)
            res["hr_drop"] = pg.evaluate("""() => { const n = document.querySelector('#fl-note'), h = document.querySelector('#hrmix'), f = ST.files.find(x => /HR_DDA/.test(x.name));
              return {tick: f ? !!f.use : null, opened: f ? E.files.some(x => !x.gone && (x.file === f.name || x.file.startsWith(f.name + '#'))) : null,
                note: n && !n.hidden ? n.textContent.trim() : '', hrmix: h && !h.hidden ? h.textContent.trim().slice(0, 200) : '', rows_before: window.__hr0, rows_after: document.querySelectorAll('#flst .fl:not(.pr)').length}; }""")
            res["render_cost"] = render_cost(pg)
            res["remove_all"] = remove_all(pg)
            pg.close()
            res["w820"] = w820(r, p, lr)
            res["return"] = return_visit(r, p, lr)
            res["first_vitals_note"] = "FCP/LCP in ms from navigation start; CLS from the first paint to idle"
            print(json.dumps(res))
            if os.environ.get("MZLAB_E2E_JSON"): Path(os.environ["MZLAB_E2E_JSON"]).write_text(json.dumps(res))
            step("first paint: cumulative layout shift below the threshold", lambda: (_ for _ in ()).throw(AssertionError(f"CLS {res['first']['cls']:.4f}")) if res["first"]["cls"] >= CLS_MAX else None)
            step("drop: the rows arrive", lambda: (_ for _ in ()).throw(AssertionError("no row after the drop")) if res["drop"]["t_last_row_ms"] is None and res["drop"]["t_last_flist_row_ms"] is None else None)
            step("820 px: no horizontal scroll on the landing and after the first panel", lambda: (_ for _ in ()).throw(AssertionError(str(res["w820"]))) if not (res["w820"]["landing"]["no_hscroll"] and res["w820"]["after_first_panel"]["no_hscroll"]) else None)
    finally:
        r.close()
    errs = [e for e in r.errs if e[0] in ("pageerror",)]
    bad = [s_ for s_ in steps if s_[1] != "ok"]
    for n_, s_ in steps: print(("OK   " if s_ == "ok" else "FAIL ") + n_ + ("" if s_ == "ok" else "  -> " + s_))
    print("page errors:", errs)
    sys.exit(1 if bad or errs else 0)
