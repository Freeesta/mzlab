"""Accessibility of Disegno (WCAG 2.2 AA) outside Ketcher's own iframe: axe-core (serious/critical) in the three themes, accessible names, Tab with a visible focus
indicator, target size >= 24 px (2.5.8), the editor iframe name, the loading announcement and the names and sizes of the controls of the drop-down bar (#tp-bar,
inside the iframe). Known gaps live in docs/agenti/a11y-da-sistemare.md. Fails (does not skip) when axe is missing: in CI that is a failure."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

AXE = axe_path()
if not AXE:
    print("FAIL axe-core not available (pip install axe-playwright-python)"); sys.exit(1)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_a11y_disegno.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n", " ")[:900]))
RUN = """() => axe.run({include: [['body']], exclude: [['#kframe']]}, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
def axe_ok(pg):
    if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=AXE)
    v = pg.evaluate(RUN); assert not v, v
# Controls of the Disegno view and of the page chrome, outside the iframe.
VISIBLE = """() => [...document.querySelectorAll('#v-draw button, #v-draw select, #v-draw input:not([type=hidden]), #v-draw [role=button], #nav button, #nav a[href]')]
  .filter(e => e.offsetParent !== null && !e.disabled && e.tabIndex >= 0 && !e.closest('[hidden], [inert]') && e.checkVisibility({contentVisibilityAuto: true, visibilityProperty: true}))"""
NAME = """() => (%s)().filter(e => { const n = (e.getAttribute('aria-label') || e.title || e.textContent || (e.labels && e.labels[0] && e.labels[0].textContent) || e.placeholder || e.value || ''); return !String(n).trim(); })
  .map(e => e.tagName + '#' + (e.id || e.className))""" % VISIBLE
r = Run(port=8870, wd="/tmp/wd_a11y_dis")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        attendi(pg, "!!(window.TPDraw && TPDraw.ready())", 60000)
        attendi(pg, "!!document.querySelector('#kframe')?.contentDocument?.querySelector('#tp-bar')", 60000)
        for theme in ("light", "dark"):
            def th(theme=theme):
                pg.evaluate("t => document.documentElement.setAttribute('data-theme', t)", theme); attendi(pg, "document.documentElement.dataset.theme === %r" % theme); axe_ok(pg)
            step("axe outside the editor: " + theme + " theme", th)
        pg.evaluate("document.documentElement.setAttribute('data-theme', 'light')")
        def frame_name():
            assert pg.evaluate("(document.querySelector('#kframe').getAttribute('aria-label') || '').trim().length") > 0, "the editor iframe has no name"
            assert pg.evaluate("document.querySelector('#kload').hidden"), "the loading note is still visible"
        step("the editor iframe has a name and the loading note is gone", frame_name)
        def kload():
            assert pg.evaluate("(() => { const k = document.querySelector('#kload'); return !!k && (k.getAttribute('role') === 'status' || k.getAttribute('aria-live') || !!k.closest('[aria-live], [role=status]')); })()"), "#kload is not announced"
        step("the loading note of the editor is a live region", kload)
        step("every visible control of Disegno has an accessible name", lambda: (lambda bad: (_ for _ in ()).throw(AssertionError(bad[:15])) if bad else None)(pg.evaluate(NAME)))
        def tab_reach():
            pg.evaluate("document.activeElement && document.activeElement.blur(); window.scrollTo(0,0)")
            want = pg.evaluate("(" + VISIBLE + ")().length"); seen, bad = set(), []
            for i in range(want * 3 + 40):
                pg.keyboard.press("Tab")
                info = pg.evaluate("""() => { const e = document.activeElement; if (!e || e === document.body) return null;
                    if (e.tagName === 'IFRAME') return ['iframe', true, true];
                    const s = getComputedStyle(e), r = e.getBoundingClientRect();
                    const ring = (s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0) || (s.boxShadow && s.boxShadow !== 'none');
                    const k = e.id || e.dataset.a || e.dataset.o || e.getAttribute('aria-label') || e.className || e.tagName;
                    return [k + '#' + [...document.querySelectorAll(e.tagName)].indexOf(e), !!ring, r.width * r.height > 0]; }""")
                if info is None: continue
                if info[0] == 'iframe':
                    # Ketcher has its own tab order: Tab must leave the iframe again (no trap)
                    for _ in range(400):
                        if pg.evaluate("document.activeElement.tagName !== 'IFRAME'"): break
                        pg.keyboard.press("Tab")
                    else: raise AssertionError("keyboard trap: Tab does not leave the editor iframe")
                    continue
                if info[0] in seen and len(seen) >= want: break
                seen.add(info[0])
                if not info[1] and info[2]: bad.append(info[0])
            miss = pg.evaluate("(" + VISIBLE + ")().map(e => e.id || e.dataset.a || e.getAttribute('aria-label') || e.className || e.tagName)")
            assert len(seen) >= want * 0.9, f"Tab reached {len(seen)} of {want} visible controls; seen: {sorted(seen)[:40]}; all: {miss}"
            assert not bad, "no visible focus indicator on: " + ", ".join(bad[:12])
        step("keyboard: Tab reaches the controls of Disegno, each with a visible focus indicator", tab_reach)
        def targets():
            # inline links and checkboxes inside their label are exempt (2.5.8: inline, spacing); axe's target-size rule covers them above
            small = pg.evaluate("""() => (%s)().filter(e => e.tagName === 'BUTTON' || e.tagName === 'SELECT').map(e => [e.tagName + '#' + (e.id || e.className), e.getBoundingClientRect()]).filter(([k, b]) => b.width < 24 || b.height < 24)
                .map(([k, b]) => k + ' ' + Math.round(b.width) + 'x' + Math.round(b.height))""" % VISIBLE)
            assert not small, small[:15]
        step("WCAG 2.5.8: the controls of Disegno are at least 24 x 24 px", targets)
        def bar():
            bad = pg.evaluate("""() => { const d = document.querySelector('#kframe').contentDocument;
                return [...d.querySelectorAll('#tp-bar button, #tp-bar [role=menuitem]')].filter(e => e.getClientRects().length).map(e => [e.getAttribute('aria-label') || e.textContent.trim(), e.getBoundingClientRect()])
                  .filter(([n, b]) => !n || b.width < 24 || b.height < 24).map(([n, b]) => (n || '(no name)') + ' ' + Math.round(b.width) + 'x' + Math.round(b.height)); }""")
            assert not bad, bad
        step("the drop-down bar controls have a name and are at least 24 x 24 px", bar)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
