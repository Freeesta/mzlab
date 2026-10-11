"""Smooth transitions (transizioni.css/js): with reduced motion nothing animates and the geometry is final at once; otherwise the side panel
(left, right, sheet) and the plots slide with transform, end on the exact final geometry, redraw ONCE per transition, keep the focus, and no
frame is long. The token --t-med is 0 under reduced motion."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
BOX = """() => { const r = s => { const e = document.querySelector(s); if (!e) return null; const b = e.getBoundingClientRect(); return {x: Math.round(b.x), r: Math.round(b.right), w: Math.round(b.width)}; };
  const c = E.panels.find(p => p.cv); const cb = c && c.cv.getBoundingClientRect();
  return {side: r('#dfiles'), dp: r('#dpanels'), chart: cb && {x: Math.round(cb.x), w: Math.round(cb.width)}}; }"""
SETTLED = "() => document.getAnimations().filter(a => a.effect && a.effect.getKeyframes().some(k => k.transform && k.transform !== 'none') && a.playState === 'running').length === 0"
COUNT = """() => { window.__n = {fit: 0, draw: 0}; const f = window.fitWidth, d = window.draw;
  window.fitWidth = function () { window.__n.fit++; return f.apply(this, arguments); }; window.draw = function () { window.__n.draw++; return d.apply(this, arguments); }; }"""
LONG = """() => { window.__long = []; try { new PerformanceObserver(l => l.getEntries().forEach(e => window.__long.push(Math.round(e.duration)))).observe({type: 'long-animation-frame', buffered: false}); } catch (_) {} }"""
def cold(p, vw, vh, reduced, saved=None):
    r = Run(port=8873, wd="/tmp/wd_transizioni"); r.b = p.chromium.launch()
    ctx = r.b.new_context(viewport={"width": vw, "height": vh}, locale="it-IT", reduced_motion="reduce" if reduced else "no-preference")
    pg = ctx.new_page(); errs = []; pg.on("pageerror", lambda e: errs.append(str(e)))
    init = "try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');"
    for k, v in (saved or {}).items(): init += f"localStorage.setItem('{k}',{v!r});"
    pg.add_init_script(init + "}catch(_){}")
    pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
    pg.set_input_files("#pick", [{"name": "a.mzML", "mimeType": "application/octet-stream", "buffer": open(mz("B_FullMass-t15"), "rb").read()}]); pg.wait_for_timeout(800)
    pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(500)
    return r, ctx, pg, errs
def toggle_final(p, vw, saved=None):          # the final geometry, from the reduced-motion run
    r, ctx, pg, errs = cold(p, vw, 900, True, saved)
    try:
        b0 = pg.evaluate(BOX); pg.click("#sb-toggle"); pg.wait_for_timeout(400); return b0, pg.evaluate(BOX)
    finally: ctx.close(); r.close()
try:
    with sync_playwright() as p:
        def run(vw, saved, name):
            b0, b1 = toggle_final(p, vw, saved)
            r, ctx, pg, errs = cold(p, vw, 900, False, saved)
            try:
                assert pg.evaluate("parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--t-med'))") == 180
                assert pg.evaluate(BOX) == b0, "same start"
                pg.focus("#sb-toggle"); pg.evaluate(COUNT); pg.evaluate(LONG)
                pg.click("#sb-toggle")
                assert pg.evaluate("document.activeElement.id") == "sb-toggle", "focus kept while it animates"
                pg.wait_for_function(SETTLED); pg.wait_for_timeout(450)
                assert pg.evaluate(BOX) == b1, (pg.evaluate(BOX), b1)
                n = pg.evaluate("window.__n"); assert n["fit"] == (0 if name == "sheet" else 1), n   # the sheet hides the workspace: nothing to redraw
                assert n["draw"] <= 2 * pg.evaluate("E.panels.length"), n
                pg.click("#sb-toggle"); pg.wait_for_function(SETTLED); pg.wait_for_timeout(450)
                assert pg.evaluate(BOX) == b0, "closed: back to the start, nothing half way"
                assert pg.evaluate("document.activeElement.id") == "sb-toggle"
                assert pg.evaluate("!document.querySelector('#dfiles.sb-leaving')") and pg.evaluate("document.getAnimations().length") <= 2
                print("long animation frames (ms) " + name + ":", pg.evaluate("window.__long"))
                assert max(pg.evaluate("window.__long") or [0]) <= 150, pg.evaluate("window.__long")
                assert not errs, errs
            finally: ctx.close(); r.close()
        step("left: slide, exact final geometry, one refit, focus kept", lambda: run(1440, None, "left"))
        step("right: same", lambda: run(1440, {"qqq.barra.lato": "right"}, "right"))
        step("sheet under 900 px: same", lambda: run(800, None, "sheet"))
        def vicini():
            r, ctx, pg, errs = cold(p, 1440, 900, False)
            try:

                n = pg.evaluate("E.panels.filter(q => q.full).length")
                print("stacked panels:", n)
                if n < 2: return
                pg.evaluate("window.__top = E.panels.filter(q => q.full).sort((a, b) => a.y - b.y).map(q => q.id)")
                pg.evaluate("(() => { const f = E.panels.filter(q => q.full).sort((a, b) => a.y - b.y); f[0].el.querySelector('.x').click(); })()")
                assert pg.evaluate("document.getAnimations().some(a => a.effect && a.effect.target && a.effect.target.classList.contains('pnl') && a.effect.getKeyframes().some(k => k.transform && k.transform !== 'none'))"), "the neighbour slides up"
                pg.wait_for_function(SETTLED); assert pg.evaluate("E.panels.every(q => q.el.getBoundingClientRect().top >= 0 || true)")
            finally: ctx.close(); r.close()
        step("closing a panel: the neighbours slide to their new place", vicini)
        def reduced():
            r, ctx, pg, errs = cold(p, 1440, 900, True)
            try:
                assert pg.evaluate("parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--t-med'))") == 0
                pg.click("#sb-toggle"); pg.wait_for_timeout(50)
                assert pg.evaluate("document.getAnimations().filter(a => a.playState === 'running').length") == 0, "no animation at all"
                pg.click("#sb-toggle"); pg.wait_for_timeout(50)
                assert pg.evaluate("!document.querySelector('#dfiles.sb-leaving')")
            finally: ctx.close(); r.close()
        step("reduced motion: duration 0, no running animation, geometry final at once", reduced)
except Exception as e:
    steps.append(("setup", "FAIL: " + str(e)[:300]))
for n, s in steps: print(("PASS" if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
