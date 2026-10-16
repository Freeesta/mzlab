"""Accessibility of the Dati view in the fast path (--fumo, mandatory CI job): axe-core (serious/critical, WCAG 2.2 AA, contrast included) on six states,
keyboard only (every visible control reachable with Tab, visible focus, Esc closes each dialog), reflow at 320 CSS px (= 400 % zoom), forced colours
and reduced motion. Known gaps live in docs/agenti/a11y-da-sistemare.md and in KNOWN below (an id there is reported but does not fail).
Unlike e2e_axe_grafici this test does NOT skip when axe is missing: in CI that is a failure."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *


AXE = axe_path()
if not AXE:
    print("FAIL axe-core not available (pip install axe-playwright-python)"); sys.exit(1)
KNOWN = set()          # axe rule ids tolerated for now (each one listed in docs/agenti/a11y-da-sistemare.md)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_a11y_dati.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n", " ")[:900]))
r = Run(port=8868, wd="/tmp/wd_a11y")
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
def axe_ok(pg, sel="body"):
    v = [x for x in pg.evaluate(RUN, sel) if x[0] not in KNOWN]; assert not v, v
VISIBLE = """() => [...document.querySelectorAll('button, select, input:not([type=hidden]), a[href], [role=button], [role=tab], [tabindex="0"], summary')]
  .filter(e => e.offsetParent !== null && !e.disabled && e.tabIndex >= 0 && !e.closest('[hidden], [inert], dialog:not([open])') && getComputedStyle(e).visibility !== 'hidden')"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.add_script_tag(path=AXE)
        step("axe: start screen (no file)", lambda: axe_ok(pg))
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15"), mz("B_MRM-t0")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(1200)
        pg.add_script_tag(path=AXE)
        step("axe: file loaded (Full Scan panels)", lambda: axe_ok(pg))
        def mrm():
            pg.evaluate("setTab('mrm')"); pg.wait_for_timeout(2500); ready(pg); axe_ok(pg)
            pg.evaluate("setTab('full')"); pg.wait_for_timeout(800)
        step("axe: MRM tab", mrm)
        def mappa():
            pg.evaluate("addPanel('map',{tab:'full'})"); pg.wait_for_timeout(2500); ready(pg); axe_ok(pg)
        step("axe: RT x m/z map", mappa)
        def settings():
            pg.click("#np-set"); pg.wait_for_selector("#uipset", state="visible"); axe_ok(pg)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
        step("axe: settings", settings)
        def search():
            pg.keyboard.press("Control+k"); pg.wait_for_selector("#paletta:not([hidden])"); axe_ok(pg)
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert pg.evaluate("document.querySelector('#paletta').hidden"), "Esc must close the search bar"
        step("axe: search bar opens with Ctrl+K and closes with Esc", search)

        def esc_closes():
            pg.evaluate("E.panels.find(p=>p.type==='chrom').cv.focus()"); pg.keyboard.press("t"); pg.wait_for_selector("#tdati[open]")
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300); assert not pg.evaluate("!!document.querySelector('#tdati[open]')"), "Esc must close the data table"
            pg.click("#np-set"); pg.wait_for_selector("#uipset", state="visible"); pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            assert pg.evaluate("(() => { const e = document.querySelector('#uipset'); return !e || e.hidden || e.offsetParent === null; })()"), "Esc must close the settings"
        step("keyboard: Esc closes the data table and the settings", esc_closes)
        def tab_reach():
            pg.evaluate("document.activeElement && document.activeElement.blur(); window.scrollTo(0,0)")
            want = pg.evaluate("(" + VISIBLE + ")().length"); seen, bad, trap = set(), [], False
            for i in range(want * 3 + 40):
                pg.keyboard.press("Tab")
                info = pg.evaluate("""() => { const e = document.activeElement; if (!e || e === document.body) return null;
                    const s = getComputedStyle(e), r = e.getBoundingClientRect();
                    const ring = (s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0) || (s.boxShadow && s.boxShadow !== 'none');
                    const k = e.id || e.dataset.a || e.dataset.o || e.getAttribute('aria-label') || e.className || e.tagName;
                    return [k + '#' + [...document.querySelectorAll(e.tagName)].indexOf(e), !!ring, r.width * r.height > 0]; }""")
                if info is None: continue
                if info[0] in seen: break
                seen.add(info[0])
                if not info[1] and info[2]: bad.append(info[0])
            assert len(seen) >= want * 0.9, f"Tab reached {len(seen)} of {want} visible controls"
            assert not bad, "no visible focus indicator on: " + ", ".join(bad[:12])
        step("keyboard: Tab reaches the visible controls, each with a visible focus indicator", tab_reach)
        def reflow():
            ctx = r.b.new_context(viewport={"width": 320, "height": 800}, locale="it-IT"); q = ctx.new_page()
            q.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');localStorage.setItem('qqq.pannelli.affianca.hint','1');}catch(_){}")
            try:
                q.goto(f"http://127.0.0.1:{r.port}/?telefono=0"); q.wait_for_timeout(1500)
                q.set_input_files("#pick", [mz("B_FullMass-t0")]); q.wait_for_timeout(800); (q.click("#opbtn") if q.locator("#opbtn").is_visible() else None); ready(q)
                w = q.evaluate("[document.documentElement.scrollWidth, document.documentElement.clientWidth]"); assert w[0] <= w[1] + 1, f"horizontal scroll at 320 px: {w}"
            finally: ctx.close()
        step("WCAG 1.4.10 reflow: 320 CSS px without horizontal page scroll", reflow)
        def forced():
            pg.emulate_media(forced_colors="active"); pg.wait_for_timeout(400)
            try:
                small = pg.evaluate("[...document.querySelectorAll('#dpanels button, #uip button')].filter(e => e.offsetParent && getComputedStyle(e).display !== 'none' && e.getBoundingClientRect().width === 0).length")
                assert small == 0, small
                assert pg.evaluate("matchMedia('(forced-colors: active)').matches"), "forced-colors emulation did not apply"
                axe_ok(pg)
            finally: pg.emulate_media(forced_colors="none")
        step("forced colours: controls keep their size and axe stays clean", forced)
        def motion():
            assert pg.evaluate("matchMedia('(prefers-reduced-motion: reduce)').matches")
            pg.click("#np-set"); pg.wait_for_selector("#uipset", state="visible"); pg.keyboard.press("Escape")
            pg.evaluate("setTab('full')"); pg.wait_for_timeout(700)
            run = pg.evaluate("document.getAnimations().filter(a => a.playState === 'running' && a.effect && a.effect.getComputedTiming().iterations === Infinity).map(a => (a.effect.target && (a.effect.target.id || a.effect.target.className)) + ':' + (a.animationName || a.transitionProperty))")
            assert not run, run
        step("prefers-reduced-motion: no looping animation after the actions", motion)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
