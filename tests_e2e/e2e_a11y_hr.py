"""Accessibility of the HR bench (WCAG 2.2 AA): axe-core (serious/critical) on the bench, the formula calculator and the information bar, accessible names on
every control, Tab reaching the visible controls with a visible focus indicator, no keyboard trap, target size >= 24 px (2.5.8) and live regions for dynamic
updates. Known gaps live in docs/agenti/a11y-da-sistemare.md. Fails (does not skip) when axe is missing: in CI that is a failure."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load

AXE = axe_path()
if not AXE:
    print("FAIL axe-core not available (pip install axe-playwright-python)"); sys.exit(1)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_a11y_hr.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n", " ")[:900]))
RUN = """(sel) => axe.run(document.querySelector(sel) || document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
def axe_ok(pg, sel="body"):
    if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=AXE)
    v = pg.evaluate(RUN, sel); assert not v, v
# Controls a user can reach: visible, enabled, not inside a hidden or inert container.
VISIBLE = """() => [...document.querySelectorAll('#dpanels button, #dpanels select, #dpanels input:not([type=hidden]), #dpanels a[href], #dpanels [role=button], #dpanels [tabindex="0"], #uip button, #dfiles button, #dfiles input, #dfiles select')]
  .filter(e => e.offsetParent !== null && !e.disabled && e.tabIndex >= 0 && !e.closest('[hidden], [inert]') && getComputedStyle(e).visibility !== 'hidden')"""
NAME = """() => (%s)().filter(e => { const n = (e.getAttribute('aria-label') || e.getAttribute('aria-labelledby') && (document.getElementById(e.getAttribute('aria-labelledby').split(' ')[0]) || {}).textContent
    || e.title || e.textContent || (e.labels && e.labels[0] && e.labels[0].textContent) || e.placeholder || e.value || ''); return !String(n).trim(); })
  .map(e => e.tagName + '#' + (e.id || e.className))""" % VISIBLE
D = synth("a11y_hr")
r = Run(port=8869, wd="/tmp/wd_a11y_hr")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 1100})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        step("axe: the HR bench", lambda: axe_ok(pg, "#dpanels"))
        step("axe: the whole page with the HR bench open", lambda: axe_ok(pg))
        def info():
            pg.evaluate("BARRA.setTab('hdr')"); attendi(pg, "(document.querySelector('#hri-body')?.innerText || '').length > 0"); axe_ok(pg, "#hrinfo")
        step("axe: the information bar of the HR bench", info)
        def calc():
            pg.click("#np-calc2"); pg.wait_for_selector("#sb-panel-calc:not([hidden])"); axe_ok(pg, "#sb-panel-calc")
        step("axe: the calculator panel", calc)
        step("every visible control of the HR bench has an accessible name", lambda: (lambda bad: (_ for _ in ()).throw(AssertionError(bad[:15])) if bad else None)(pg.evaluate(NAME)))
        def tab_reach():
            pg.evaluate("document.activeElement && document.activeElement.blur(); window.scrollTo(0,0)")
            want = pg.evaluate("(" + VISIBLE + ")().length"); seen, bad = set(), []
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
        step("keyboard: Tab reaches the HR controls, each with a visible focus indicator (and cycles: no trap)", tab_reach)
        def targets():
            small = pg.evaluate("""() => (%s)().map(e => [e.tagName + '#' + (e.id || e.className), e.getBoundingClientRect()]).filter(([k, b]) => b.width < 24 || b.height < 24)
                .map(([k, b]) => k + ' ' + Math.round(b.width) + 'x' + Math.round(b.height))""" % VISIBLE)
            assert not small, small[:15]
        step("WCAG 2.5.8: the HR controls are at least 24 x 24 px", targets)
        def live():
            n = pg.evaluate("[...document.querySelectorAll('[aria-live], [role=status], [role=alert]')].length")
            assert n >= 3, f"only {n} live regions on the page"
        step("dynamic updates: live regions exist", live)
        def esc():
            pg.keyboard.press("Escape"); attendi(pg, "!document.querySelector('dialog[open]')")
        step("keyboard: Esc leaves no open dialog", esc)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
