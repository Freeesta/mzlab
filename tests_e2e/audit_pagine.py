"""Page audit (docs/studi/AUDIT_PAGINE.md): every view x {no samples, 1 sample, 5 samples} x {cold, warm return}; every header button opened
and closed; panels added and closed; CLS, panel movement, untranslated text, console errors, requests, memory over 20 open/close cycles.
Not a test (no pass/fail): it writes audit.json and screenshots into OUT_DIR. Needs the built site (tools/prepara_sito.py).

    MZLAB_MZML=.verifica/dati_sintetici python3 tests_e2e/audit_pagine.py OUT_DIR [--quick]     (or the real lab files)
"""
import json, os, sys, time
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import Run, stage, open_first, ready, frames, hold, mz
from playwright.sync_api import sync_playwright

OUT = Path(next((a for a in sys.argv[1:] if not a.startswith("--")), "audit_pagine"))
OUT.mkdir(parents=True, exist_ok=True)
QUICK = "--quick" in sys.argv

INSTR = r"""(() => { if (window.__au) return; const au = window.__au = {cls: 0, shifts: [], t0: performance.now(), fcp: null, untr: []};
 try { new PerformanceObserver(l => { for (const e of l.getEntries()) { if (e.hadRecentInput) continue; au.cls += e.value;
   au.shifts.push({t: Math.round(e.startTime), v: +e.value.toFixed(4), src: (e.sources || []).slice(0, 3).map(s => { const n = s.node; if (!n || !n.nodeType) return '?'; const el = n.nodeType === 1 ? n : n.parentElement; return el ? (el.id ? '#' + el.id : el.tagName.toLowerCase() + (el.className && typeof el.className === 'string' ? '.' + el.className.trim().split(/\s+/).slice(0,2).join('.') : '')) + ' ' + JSON.stringify([Math.round(s.previousRect.x), Math.round(s.previousRect.y), Math.round(s.currentRect.x), Math.round(s.currentRect.y)]) : '?'; })}); } }).observe({type: 'layout-shift', buffered: true}); } catch (_) {}
 try { new PerformanceObserver(l => { for (const e of l.getEntries()) if (e.name === 'first-contentful-paint') au.fcp = Math.round(e.startTime); }).observe({type: 'paint', buffered: true}); } catch (_) {}
})()"""

# geometry of the main blocks: what must not move when a popup opens or a view comes back
GEOM = r"""() => { const r = s => { const e = document.querySelector(s); if (!e || !e.offsetParent && getComputedStyle(e).position !== 'fixed') return null; const b = e.getBoundingClientRect(); return [Math.round(b.x), Math.round(b.y), Math.round(b.width), Math.round(b.height)]; };
 const o = {}; for (const s of ['header', '#nav', '#stbar', '#sb-pane', '#v-workspace', '#tools', '#start', '#v-draw', '#flst']) o[s] = r(s);
 (window.E && E.panels || []).forEach((p, i) => { if (p.el && p.el.isConnected) { const b = p.el.getBoundingClientRect(); o['panel' + i + ':' + p.type] = [Math.round(b.x), Math.round(b.y), Math.round(b.width), Math.round(b.height)]; } });
 return o; }"""

# what is open: dialogs, menus, popovers, expanded buttons, sidebar tab
OPEN = r"""() => { const vis = e => { if (!e.isConnected || e.hidden) return false; const cs = getComputedStyle(e); if (cs.display === 'none' || cs.visibility === 'hidden' || +cs.opacity === 0) return false; const b = e.getBoundingClientRect(); return b.width > 2 && b.height > 2; };
 const lab = e => e.id ? '#' + e.id : e.tagName.toLowerCase() + (typeof e.className === 'string' && e.className ? '.' + e.className.trim().split(/\s+/)[0] : '');
 const dlg = [...document.querySelectorAll('dialog[open], [role=dialog], [role=menu], [role=listbox], .pop, .modal, .ov, #ctx, #palette, .cmdp, #uip, .uip, .tour-pop, [popover], #helppop, #calcdlg')].filter(vis).map(lab);
 const exp = [...document.querySelectorAll('[aria-expanded=true]')].filter(vis).map(lab);
 const pressed = [...document.querySelectorAll('header [aria-pressed=true]')].map(lab);
 const sb = document.querySelector('#sb-pane'); return {dlg: [...new Set(dlg)].sort(), exp: exp.sort(), pressed: pressed.sort(), sb: sb && vis(sb) ? (window.BARRA ? String(BARRA.curTab) : 'open') : null, view: window.S && S.view, fs: !!document.fullscreenElement}; }"""

# the visible header buttons, with a selector that finds them again
HDR = r"""() => { const vis = e => { const b = e.getBoundingClientRect(); return b.width > 0 && b.height > 0 && getComputedStyle(e).visibility !== 'hidden'; };
 return [...document.querySelectorAll('header button, header [role=button]')].filter(vis).map(b => {
  let s = b.id ? '#' + b.id : b.dataset.cmd ? `header [data-cmd="${b.dataset.cmd}"]` : b.dataset.cmdMain ? `header [data-cmd-main="${b.dataset.cmdMain}"]` : b.dataset.cmdCar ? `header [data-cmd-car="${b.dataset.cmdCar}"]` : b.dataset.v ? `#nav [data-v="${b.dataset.v}"]` : b.dataset.help != null ? `header .hq[data-help="${b.dataset.help}"]` : null;
  return {sel: s, name: (b.getAttribute('aria-label') || b.textContent || '').trim().slice(0, 40), nav: !!b.closest('#nav'), dis: b.disabled}; }).filter(x => x.sel); }"""

# visible text that looks like an untranslated key (a.b.c) or a placeholder left in
UNTR = r"""() => { const out = []; const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT); let n;
 while ((n = w.nextNode())) { const t = n.nodeValue.trim(); if (!t) continue; const el = n.parentElement; if (!el || el.closest('script,style,svg,code,pre,.mono')) continue;
  if (/^[a-z][a-zA-Z0-9]*(\.[a-zA-Z0-9_]+){2,}$/.test(t) || /\{[a-zA-Z]+\}/.test(t)) { const b = el.getBoundingClientRect(); if (b.width && b.height) out.push(t.slice(0, 60)); } }
 for (const e of document.querySelectorAll('[title],[aria-label],[placeholder]')) for (const a of ['title', 'aria-label', 'placeholder']) { const v = e.getAttribute(a); if (v && (/^[a-z][a-zA-Z0-9]*(\.[a-zA-Z0-9_]+){2,}$/.test(v) || /\{[a-zA-Z]+\}/.test(v))) out.push(a + '=' + v.slice(0, 60)); }
 return [...new Set(out)].slice(0, 20); }"""

# frames sampled while something animates: did any main block move between frames?
MOTION = r"""(ms) => new Promise(ok => { const sels = ['#sb-pane', '#v-workspace', '#tools', '#start', 'header']; const g = () => sels.map(s => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return [Math.round(b.x), Math.round(b.y), Math.round(b.width)]; });
 const first = g(); const moves = {}; const t0 = performance.now(); const f = () => { const c = g(); c.forEach((v, i) => { if (v && first[i] && (v[0] !== first[i][0] || v[1] !== first[i][1] || v[2] !== first[i][2])) (moves[sels[i]] = moves[sels[i]] || []).push(v); }); if (performance.now() - t0 < ms) requestAnimationFrame(f); else ok({moves: Object.fromEntries(Object.entries(moves).map(([k, v]) => [k, [first[sels.indexOf(k)], v[0], v[v.length - 1], v.length]])), anim: document.getAnimations().filter(a => a.playState === 'running').map(a => (a.animationName || a.transitionProperty || a.constructor.name) + '@' + (a.effect && a.effect.target ? (a.effect.target.id || a.effect.target.className || a.effect.target.tagName) : '')).slice(0, 10)}); };
 requestAnimationFrame(f); })"""


class Cell:
    def __init__(s, name): s.name, s.d = name, {"name": name, "errs": [], "reqs": [], "buttons": [], "notes": []}


def metrics(cdp):
    cdp.send("HeapProfiler.collectGarbage")
    m = {x["name"]: x["value"] for x in cdp.send("Performance.getMetrics")["metrics"]}
    return {k: int(m.get(k, 0)) for k in ("Nodes", "JSEventListeners", "JSHeapUsedSize", "Documents", "Frames")}


def settle(pg):
    try: ready(pg, timeout=30000, settle=300)
    except Exception as e: return str(e)[:200]
    frames(pg, 3)


def close_everything(pg, base):
    """Escape, then the side panel put back as it was: back to the base state."""
    for _ in range(3):
        if pg.evaluate(OPEN) == base: return True
        pg.keyboard.press("Escape"); frames(pg, 3)
        if document_fs(pg): pg.evaluate("document.exitFullscreen()"); frames(pg, 3)
    pg.evaluate("sb => { if (!window.BARRA) return; if (sb) BARRA.show(sb === 'open' ? 'file' : sb, {noFocus: true}); else BARRA.close(); }", base["sb"])
    settle(pg)
    return pg.evaluate(OPEN) == base


def document_fs(pg): return pg.evaluate("!!document.fullscreenElement")


def walk_header(pg, cell, cdp, shots):
    base = pg.evaluate(OPEN)
    g0 = pg.evaluate(GEOM)
    btns = pg.evaluate(HDR)
    for b in btns:
        if b["nav"] or b["dis"]: continue
        rec = {"sel": b["sel"], "name": b["name"]}
        try:
            cls0 = pg.evaluate("__au.cls")
            n0 = len(pg.context.pages)
            pg.click(b["sel"], timeout=5000)
            mo = pg.evaluate(MOTION, 700)
            settle(pg)
            st = pg.evaluate(OPEN)
            rec["opened"] = {k: v for k, v in st.items() if v != base.get(k)}
            rec["motion_open"] = mo
            rec["untr"] = pg.evaluate(UNTR)
            if shots and rec["opened"]:
                pg.screenshot(path=str(OUT / f"{cell.name}__{b['sel'].strip('#').replace(' ', '_').replace('[', '').replace(']', '').replace('=', '-').replace('\"', '')}.png"))
            if len(pg.context.pages) > n0:
                for p in pg.context.pages[n0:]: rec.setdefault("popup", []).append(p.url); p.close()
            # close: same button again, then Escape
            ok = st == base
            if not ok:
                pg.keyboard.press("Escape"); frames(pg, 3); settle(pg)
                ok = pg.evaluate(OPEN) == base
                rec["closed_by"] = "esc" if ok else None
            if not ok:
                try:
                    pg.click(b["sel"], timeout=3000); settle(pg); ok = pg.evaluate(OPEN) == base
                    if ok: rec["closed_by"] = "toggle"
                except Exception as e: rec["close_err"] = str(e)[:120]
            if not ok:
                rec["left_open"] = {k: v for k, v in pg.evaluate(OPEN).items() if v != base.get(k)}
                rec["restored"] = close_everything(pg, base)
            g1 = pg.evaluate(GEOM)
            rec["geom_changed"] = {k: [g0.get(k), g1.get(k)] for k in set(g0) | set(g1) if g0.get(k) != g1.get(k)}
            rec["cls"] = round(pg.evaluate("__au.cls") - cls0, 4)
        except Exception as e:
            rec["err"] = str(e)[:200]
            close_everything(pg, base)
        cell.d["buttons"].append(rec)
    return btns


def cycles(pg, cdp, sel, n=20):
    """Open and close one header thing n times: memory, nodes, listeners."""
    base = pg.evaluate(OPEN)
    pg.click(sel, timeout=5000); frames(pg, 2); close_everything(pg, base)       # the first opening builds what is built once
    m0 = metrics(cdp)
    for i in range(n):
        pg.click(sel, timeout=5000); frames(pg, 2)
        if not close_everything(pg, base): return {"sel": sel, "err": f"not closed at cycle {i}: {pg.evaluate(OPEN)}"}
    settle(pg)
    m1 = metrics(cdp)
    return {"sel": sel, "n": n, "before": m0, "after": m1, "delta": {k: m1[k] - m0[k] for k in m0}}


PANEL_BTNS = ["#np-chrom", "#np-spec", "#np-map", "#np-xic", "#np-mrm"]
def panels(pg):
    """Add every kind of panel from the work-area toolbar, then close it: what moved, CLS, what was left behind."""
    out = []
    for sel in PANEL_BTNS + ["#samptab"]:
        rec = {"sel": sel}
        try:
            if not pg.is_visible(sel) or pg.is_disabled(sel): rec["skip"] = "hidden/disabled"; out.append(rec); continue
            n0 = pg.evaluate("E.panels.length"); g0 = pg.evaluate(GEOM); cls0 = pg.evaluate("__au.cls"); dlg0 = pg.evaluate(OPEN)
            pg.click(sel, timeout=5000); mo = pg.evaluate(MOTION, 700); settle(pg)
            rec["panels"] = [n0, pg.evaluate("E.panels.length")]; rec["opened"] = {k: v for k, v in pg.evaluate(OPEN).items() if v != dlg0.get(k)}
            g1 = pg.evaluate(GEOM)
            rec["moved"] = {k: [g0[k], g1[k]] for k in g0 if k in g1 and g0[k] != g1[k] and k.startswith(("panel", "#sb-pane", "#tools"))}
            rec["motion"] = mo; rec["cls"] = round(pg.evaluate("__au.cls") - cls0, 4); rec["untr"] = pg.evaluate(UNTR)
            pg.screenshot(path=str(OUT / f"panel_{sel.strip('#')}_{int(time.time())}.png"))
            if sel == "#samptab": pg.click(sel); settle(pg)
            elif rec["panels"][1] > n0:
                pg.evaluate("n => { const p = E.panels[E.panels.length - 1]; const x = p.el.querySelector('.x'); if (x) x.click(); else window.__noclose = true; }", n0)
                settle(pg)
            else: pg.keyboard.press("Escape"); settle(pg)
            rec["after"] = pg.evaluate("E.panels.length"); rec["noclose"] = pg.evaluate("!!window.__noclose")
            g2 = pg.evaluate(GEOM); rec["moved_after_close"] = {k: [g0[k], g2.get(k)] for k in g0 if k.startswith(("panel", "#sb-pane", "#tools")) and g0[k] != g2.get(k)}
        except Exception as e: rec["err"] = str(e)[:200]
        out.append(rec)
    return out


def new_page(p, st):
    b = p.chromium.launch(args=["--enable-precise-memory-info"])
    pg = b.new_page(viewport={"width": 1440, "height": 900}, reduced_motion="no-preference")
    pg.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
    pg.add_init_script(INSTR)
    st["errs"], st["reqs"] = [], []
    pg.on("pageerror", lambda e: st["errs"].append(("pageerror", str(e)[:300])))
    pg.on("console", lambda m: st["errs"].append(("console." + m.type, m.text[:300])) if m.type in ("error", "warning") and "GL Driver" not in m.text else None)
    pg.on("requestfailed", lambda r: st["errs"].append(("reqfail", r.url[-80:], r.failure)))
    def resp(r):
        try:
            ln = int(r.headers.get("content-length") or 0)
        except Exception: ln = 0
        st["reqs"].append((r.status, r.url.split("/", 3)[-1][:90], ln))
    pg.on("response", resp)
    cdp = pg.context.new_cdp_session(pg); cdp.send("Performance.enable")
    return b, pg, cdp


def view(pg, v):
    t0 = time.time()
    pg.click(f'#nav [data-v="{v}"]', timeout=10000)
    mo = pg.evaluate(MOTION, 900)
    settle(pg)
    if v != "data": first_useful(pg, v)
    return round((time.time() - t0) * 1000), mo


def first_useful(pg, v):
    """ms from navigation start until the view shows its useful content."""
    sel = {"data": "#start:not([hidden]) #drop, #v-workspace .panel, #tools", "draw": "#v-draw:not([hidden]) iframe, #v-draw:not([hidden]) canvas", "tpmine": "#v-tpmine:not([hidden]) button", "flow": "#v-flow:not([hidden]) button", "theory": "#v-theory:not([hidden]) iframe"}[v]
    try:
        pg.wait_for_selector(sel, state="visible", timeout=60000)
        return pg.evaluate("Math.round(performance.now())")
    except Exception as e: return "timeout"


def main():
    r = Run(fresh=True)
    res = []
    samples = {"none": [], "one": [mz("B_FullMass-t15")], "multi": [mz(f) for f in lib.FILES]}
    views = ["data", "draw", "tpmine", "flow", "theory"]
    if QUICK: samples = {"none": [], "one": samples["one"]}
    try:
        with sync_playwright() as p:
            for sk, files in samples.items():
                st = {}
                b, pg, cdp = new_page(p, st)
                c = Cell(f"{sk}"); res.append(c.d)
                t0 = time.time()
                pg.goto(f"http://127.0.0.1:{r.port}/#/data")
                c.d["fcp"] = pg.evaluate("new Promise(ok => { const w = () => __au.fcp != null ? ok(__au.fcp) : setTimeout(w, 50); w(); })")
                c.d["useful_data"] = first_useful(pg, "data")
                pg.wait_for_function("typeof AVVIO !== 'undefined' && AVVIO.stato === 'ready'", timeout=120000)
                c.d["engine_ready_ms"] = pg.evaluate("Math.round(performance.now())")
                c.d["untr_cold"] = pg.evaluate(UNTR)
                if files:
                    tl = time.time(); stage(pg, files); open_first(pg); settle(pg)
                    c.d["load_ms"] = round((time.time() - tl) * 1000)
                settle(pg)
                c.d["cls_cold"] = round(pg.evaluate("__au.cls"), 4); c.d["shifts_cold"] = pg.evaluate("__au.shifts.slice(-12)")
                pg.screenshot(path=str(OUT / f"{sk}__data_cold.png"))
                c.d["metrics_start"] = metrics(cdp)
                # header on Data
                hc = Cell(f"{sk}_data"); walk_header(pg, hc, cdp, True); c.d["header_data"] = hc.d["buttons"]
                if files: c.d["panels"] = panels(pg)
                # page switches: cold (first visit) then warm (second)
                c.d["views"] = {}
                for v in views[1:]:
                    vv = {}
                    g0 = pg.evaluate(GEOM); cls0 = pg.evaluate("__au.cls")
                    vv["cold_ms"], vv["cold_motion"] = view(pg, v)
                    vv["cls_cold"] = round(pg.evaluate("__au.cls") - cls0, 4)
                    vv["untr"] = pg.evaluate(UNTR)
                    pg.screenshot(path=str(OUT / f"{sk}__{v}_cold.png"))
                    hc = Cell(f"{sk}_{v}"); walk_header(pg, hc, cdp, True); vv["header"] = hc.d["buttons"]
                    cls0 = pg.evaluate("__au.cls")
                    vv["back_data_ms"], vv["back_data_motion"] = view(pg, "data")
                    vv["back_cls"] = round(pg.evaluate("__au.cls") - cls0, 4)
                    vv["back_shifts"] = [x for x in pg.evaluate("__au.shifts") if x["t"] > 0][-6:]
                    g1 = pg.evaluate(GEOM)
                    vv["back_geom_changed"] = {k: [g0.get(k), g1.get(k)] for k in set(g0) | set(g1) if g0.get(k) != g1.get(k)}
                    cls0 = pg.evaluate("__au.cls")
                    vv["warm_ms"], vv["warm_motion"] = view(pg, v)
                    vv["cls_warm"] = round(pg.evaluate("__au.cls") - cls0, 4)
                    view(pg, "data")
                    c.d["views"][v] = vv
                # cold load of each non-data route by address
                c.d["direct"] = {}
                # memory: 20 cycles of a few openers
                c.d["cycles"] = []
                for sel in ['header [data-cmd="file"]', 'header [data-cmd="calc"]', '#np-set', 'header [data-cmd="search"]', '#sb-toggle', 'header [data-cmd="more"]']:
                    try: c.d["cycles"].append(cycles(pg, cdp, sel, 20))
                    except Exception as e: c.d["cycles"].append({"sel": sel, "err": str(e)[:150]})
                try:
                    m0 = metrics(cdp)
                    for i in range(10): view(pg, "draw" if i % 2 == 0 else "data")
                    view(pg, "data"); m1 = metrics(cdp)
                    c.d["cycles"].append({"sel": "views data<->draw x5", "delta": {k: m1[k] - m0[k] for k in m0}})
                except Exception as e: c.d["cycles"].append({"sel": "views", "err": str(e)[:150]})
                c.d["metrics_end"] = metrics(cdp)
                c.d["errs"], c.d["reqs_big"] = st["errs"][:60], sorted([q for q in st["reqs"] if q[2] > 500000], key=lambda q: -q[2])[:15]
                c.d["reqs_n"], c.d["reqs_bytes"] = len(st["reqs"]), sum(q[2] for q in st["reqs"])
                c.d["reqs_bad"] = [q for q in st["reqs"] if q[0] >= 400]
                b.close()
                # direct cold loads of the other routes
                for v in ("draw",):
                    st2 = {}
                    b, pg, cdp = new_page(p, st2)
                    pg.goto(f"http://127.0.0.1:{r.port}/#/{v}")
                    d = {"useful": first_useful(pg, v)}
                    settle(pg); d["cls"] = round(pg.evaluate("__au.cls"), 4); d["shifts"] = pg.evaluate("__au.shifts.slice(-8)"); d["errs"] = st2["errs"][:20]
                    pg.screenshot(path=str(OUT / f"{sk}__{v}_direct.png"))
                    c.d["direct"][v] = d
                    b.close()
                    break
                Path(OUT / "audit.json").write_text(json.dumps(res, indent=1))
    finally:
        r.close()
    Path(OUT / "audit.json").write_text(json.dumps(res, indent=1))
    print("done", OUT / "audit.json")


main()
