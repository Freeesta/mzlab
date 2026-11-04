"""Draw tab on tablets: no scroll jump on open, no keyboard trap from Ketcher's hidden textarea, 16 px fields (no iOS zoom), a reachable top bar in split view, stable rotation."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_disegno_tablet_ketcher.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
r = Run(port=8861, wd="/tmp/wd_disegno_tablet_k")
K = "document.querySelector('#kframe').contentWindow.ketcher"
INIT = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
READY = "document.getElementById('kframe').classList.contains('kwait') === false && !!(window.TPDraw && TPDraw.ready())"
def open_draw(ctx):
    pg = ctx.new_page(); pg.add_init_script(INIT)
    pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "!!document.querySelector('#nav [data-v=draw]')")
    pg.click("#nav [data-v=draw]"); attendi(pg, READY, timeout=60000); hold(pg, 2000)
    return pg
TOPBOX = "(() => { const f = document.getElementById('kframe'), t = f.contentDocument.querySelector('[class*=App-module_top]'); const b = t.getBoundingClientRect(), fb = f.getBoundingClientRect(); return {y: b.top + fb.top, bottom: b.bottom + fb.top, vh: innerHeight}; })()"
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        def scroll_top():
            for w, h, touch in ((820, 1180, True), (1180, 820, True), (768, 1024, True), (1024, 700, False)):
                ctx = r.b.new_context(viewport={"width": w, "height": h}, has_touch=touch, is_mobile=touch)
                pg = open_draw(ctx)
                sy = pg.evaluate("scrollY"); assert sy == 0, f"{w}x{h}: scrollY {sy}"
                b = pg.evaluate(TOPBOX); assert b["y"] >= 0 and b["bottom"] <= b["vh"], f"{w}x{h}: top bar not in view {b}"
                ctx.close()
        step("1 scrollY 0 and top bar in view", scroll_top)
        ctx = r.b.new_context(viewport={"width": 820, "height": 1180}, has_touch=True, is_mobile=True)
        pg = open_draw(ctx)
        def tabs():
            ca = pg.evaluate("(() => { const c = document.getElementById('kframe').contentDocument.querySelector('.cliparea'); return c && [c.getAttribute('tabindex'), c.getAttribute('inputmode')]; })()")
            assert ca == ["-1", "none"], f"cliparea attributes {ca}"
            pg.evaluate("document.getElementById('kframe').contentDocument.body.focus()")
            for _ in range(80):
                pg.keyboard.press("Tab")
                bad = pg.evaluate("(() => { const f = document.getElementById('kframe'); const a = f.contentDocument.activeElement; return !!(a && a.classList.contains('cliparea')); })()")
                assert not bad, "Tab reached .cliparea"
        step("2 cliparea is not a Tab stop", tabs)
        def fields():
            for sel in ("#ex-smi", "#ex-name"):
                fs = pg.evaluate(f"parseFloat(getComputedStyle(document.querySelector('{sel}')).fontSize)"); assert fs >= 16, f"{sel} font {fs}"
            at = pg.evaluate("(() => { const e = document.querySelector('#ex-smi'); return ['autocapitalize', 'autocorrect', 'spellcheck', 'enterkeyhint'].map(a => e.getAttribute(a)); })()")
            assert at == ["off", "off", "false", "go"], f"ex-smi attrs {at}"
            assert pg.evaluate("document.querySelector('#ex-name').getAttribute('autocapitalize')") == "off"
        step("3 fields 16 px and keyboard hints", fields)
        # 5 first, on the same page: rotation keeps the structure and the zoom
        def rotate():
            pg.evaluate(f"{K}.setMolecule('c1ccccc1')"); hold(pg, 500)
            n0 = pg.evaluate(f"{K}.editor.struct().atoms.size"); z0 = pg.evaluate(f"{K}.editor.zoom()")
            pg.set_viewport_size({"width": 1180, "height": 820}); hold(pg, 800)
            pg.set_viewport_size({"width": 820, "height": 1180}); hold(pg, 800)
            assert pg.evaluate(f"{K}.editor.struct().atoms.size") == n0, "structure changed"
            z1 = pg.evaluate(f"{K}.editor.zoom()"); assert abs(z1 - z0) < 1e-6, f"zoom {z0} -> {z1}"
            assert pg.evaluate("scrollY") == 0, "scrollY after rotation"
        step("5 rotation", rotate)
        ctx.close()
        def split():
            for w in (375, 440):
                ctx = r.b.new_context(viewport={"width": w, "height": 1024}, screen={"width": 1024, "height": 1366}, has_touch=True, is_mobile=True)
                pg = open_draw(ctx); cdp = ctx.new_cdp_session(pg)
                n = pg.evaluate("(() => { const d = document.getElementById('kframe').contentDocument, t = d.querySelector('[class*=App-module_top]'); return [...t.querySelectorAll('button')].filter(b => b.offsetParent).length; })()")
                assert n > 8, f"{w}: only {n} buttons"
                miss = pg.evaluate("""(() => { const f = document.getElementById('kframe'), d = f.contentDocument, W = d.defaultView.innerWidth, out = [];
                  for (const b of d.querySelector('[class*=App-module_top]').querySelectorAll('button')) { if (!b.offsetParent) continue; b.scrollIntoView({inline: 'center', block: 'nearest'});
                    const x = b.getBoundingClientRect(); if (x.left < -1 || x.right > W + 1 || x.width < 1) out.push(b.getAttribute('data-testid') || b.title || '?'); }
                  return out; })()""")
                assert not miss, f"{w}: unreachable {miss}"
                pg.evaluate("(() => { const t = document.getElementById('kframe').contentDocument.querySelector('[class*=App-module_top]'); t.scrollLeft = 0; })()")
                pg.evaluate(f"(() => {{ {K}.editor.tool('bond', {{type: 1, stereo: 0}}); return 1; }})()")
                fb = pg.evaluate("(() => { const r = document.getElementById('kframe').getBoundingClientRect(); return [r.left + r.width / 2, r.top + 500]; })()")
                def T(kind, pts): cdp.send("Input.dispatchTouchEvent", {"type": kind, "touchPoints": [{"x": x, "y": y, "id": 0} for x, y in pts]})
                T("touchStart", [(fb[0], fb[1])]); hold(pg, 60)
                for k in range(1, 9): T("touchMove", [(fb[0] + 12 * k, fb[1] - 5 * k)]); hold(pg, 30)
                T("touchEnd", []); hold(pg, 400)
                nb = lambda: pg.evaluate(f"{K}.editor.struct().bonds.size")
                assert nb() >= 1, f"{w}: no bond drawn"
                c = pg.evaluate("""(() => { const f = document.getElementById('kframe'), b = [...f.contentDocument.querySelectorAll('[class*=App-module_top] [data-testid=undo]')].find(e => e.getBoundingClientRect().width > 0); b.scrollIntoView({inline: 'center', block: 'nearest'});
                  const x = b.getBoundingClientRect(), fb = f.getBoundingClientRect(), z = parseFloat(getComputedStyle(f.contentDocument.documentElement).zoom) || 1; return [fb.left + x.left + x.width / 2, fb.top + x.top + x.height / 2]; })()""")
                T("touchStart", [tuple(c)]); hold(pg, 50); T("touchEnd", []); hold(pg, 400)
                assert nb() == 0, f"{w}: touch tap on undo did not remove the bond"
                ctx.close()
        step("4 split view top bar", split)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(n, s)
print("FAIL" if bad else "OK")
sys.exit(1 if bad else 0)
