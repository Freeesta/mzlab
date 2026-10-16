"""Right click on a peak label of a low-resolution Full Scan spectrum whose m/z is the precursor of MS2 scans: «Estrai la scansione MS2» opens that MS2 spectrum."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8853, wd="/tmp/wd_estrai")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MS2-t15")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("setTab('full', true)"); pg.wait_for_timeout(600)
        S = "E.panels.find(q => q.type === 'spec' && q.tab === 'full')"
        def lab_at(mzv, rt=7.07):
            pg.evaluate(f"(() => {{ const f = E.files.find(f => /t15/.test(f.label) && f.kind === 'full'); activeFile(f.k); const s = {S}; s.k = f.k; s.r0 = {rt} - 0.07; s.r1 = {rt} + 0.07; s.link = null; s.zoom = null; draw(s); }})()"); ready(pg)
            return pg.evaluate(f"(() => {{ const s = {S}, l = (s._a.lbls || []).filter(b => b.m != null && Math.abs(b.m - {mzv}) < 0.6).sort((u, v) => Math.abs(u.m - {mzv}) - Math.abs(v.m - {mzv}))[0]; if (!l) return null; const c = s.el.querySelector('canvas').getBoundingClientRect(); return [c.left + l.x + l.w / 2, c.top + l.y + l.h / 2, l.m]; }})()")
        def menu_items(pt):
            pg.mouse.click(pt[0], pt[1], button="right"); pg.wait_for_selector("#ctx:not([hidden])")
            return pg.evaluate("[...document.querySelectorAll('#ctx [role^=menuitem]')].map(e => e.textContent.trim())")
        def with_ms2():
            pt = lab_at(229.4); assert pt, "no label near 229.4"
            it = menu_items(pt); assert "Estrai la scansione MS2" in it, it
            pg.click("#ctx [role^=menuitem]:has-text('Estrai la scansione MS2')"); ready(pg); pg.wait_for_timeout(800)
            ms2 = pg.evaluate("(() => { const s = E.panels.find(q => q.type === 'spec' && q.tab === 'ms2'); return s && [s.level, s.r0, s.r1, String(s.prec)]; })()")
            assert ms2 and ms2[0] == 2 and ms2[3].startswith("229"), ms2
        step("a precursor peak offers the MS2 scan and opens its spectrum", with_ms2)
        def without():
            pg.keyboard.press("Escape")
            pt = lab_at(364.4, 14.3); assert pt, "no label near 364.4"
            menu_items(pt)
            assert pg.evaluate("(() => { const e = [...document.querySelectorAll('#ctx [role^=menuitem]')].find(x => x.textContent.trim() === 'Estrai la scansione MS2'); return !e || e.getAttribute('aria-disabled') === 'true' || e.disabled; })()"), "offered for a peak that is not a precursor"      # the action stays in the menu, grey
            pg.keyboard.press("Escape")
        step("a peak that is not a precursor does not (grey)", without)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
