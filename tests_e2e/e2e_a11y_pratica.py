"""Accessibility of Pratica (teoria/pratica*.html, WCAG 2.2 AA): on every page, in the light and the dark scheme, axe-core (serious/critical), accessible names on every
visible control, Tab reaching the controls with a visible focus indicator and without a trap, and button / select targets of at least 24 x 24 px (2.5.8).
Known gaps live in docs/agenti/a11y-da-sistemare.md. Fails (does not skip) when axe is missing: in CI that is a failure."""
import sys, os, glob; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

AXE = axe_path()
if not AXE:
    print("FAIL axe-core not available (pip install axe-playwright-python)"); sys.exit(1)
PAGES = sorted(os.path.basename(f)[:-5] for f in glob.glob(str(ROOT / "mzlab" / "web" / "teoria" / "pratica*.html")))
assert PAGES, "no Pratica page found"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_a11y_pratica.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).replace("\n", " ")[:900]))
RUN = """() => axe.run(document, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}})
  .then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length, v.nodes.slice(0, 3).map(n => n.target.join(' ')).join(' | ')]))"""
VISIBLE = """() => [...document.querySelectorAll('button, select, input:not([type=hidden]), a[href], [role=button], [tabindex="0"], summary')]
  .filter(e => e.offsetParent !== null && !e.disabled && e.tabIndex >= 0 && !e.closest('[hidden], [inert]') && e.checkVisibility({contentVisibilityAuto: true, visibilityProperty: true}))"""
NAME = """() => (%s)().filter(e => { const n = (e.getAttribute('aria-label') || e.getAttribute('aria-labelledby') && (document.getElementById(e.getAttribute('aria-labelledby').split(' ')[0]) || {}).textContent
    || e.title || e.textContent || (e.labels && e.labels[0] && e.labels[0].textContent) || e.placeholder || e.value || ''); return !String(n).trim(); })
  .map(e => e.tagName + '#' + (e.id || e.className))""" % VISIBLE
def axe_ok(pg):
    if not pg.evaluate("typeof axe !== 'undefined'"): pg.add_script_tag(path=AXE)
    v = pg.evaluate(RUN); assert not v, v
def names(pg):
    bad = pg.evaluate(NAME); assert not bad, bad[:15]
def tabs(pg):
    pg.evaluate("document.activeElement && document.activeElement.blur(); window.scrollTo(0,0)")
    want = pg.evaluate("(" + VISIBLE + ")().length"); seen, bad = set(), []
    assert want > 0, "no visible control found: the test would prove nothing"
    for i in range(want * 2 + 30):
        pg.keyboard.press("Tab")
        info = pg.evaluate("""() => { const e = document.activeElement; if (!e || e === document.body) return null;
            const s = getComputedStyle(e), r = e.getBoundingClientRect();
            const ring = (s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0) || (s.boxShadow && s.boxShadow !== 'none');
            return [e.tagName + '#' + (e.id || e.className) + '#' + [...document.querySelectorAll(e.tagName)].indexOf(e), !!ring, r.width * r.height > 0]; }""")
        if info is None: continue
        if info[0] in seen and len(seen) >= want: break
        seen.add(info[0])
        if not info[1] and info[2]: bad.append(info[0])
    assert len(seen) >= want * 0.9, f"Tab reached {len(seen)} of {want} visible controls"
    assert not bad, "no visible focus indicator on: " + ", ".join(bad[:12])
def targets(pg):
    small = pg.evaluate("""() => (%s)().filter(e => e.tagName === 'BUTTON' || e.tagName === 'SELECT').map(e => [e.tagName + '#' + (e.id || e.className), e.getBoundingClientRect()])
        .filter(([k, b]) => b.width < 24 || b.height < 24).map(([k, b]) => k + ' ' + Math.round(b.width) + 'x' + Math.round(b.height))""" % VISIBLE)
    assert not small, small[:15]
r = Run(port=8871, wd="/tmp/wd_a11y_pra")
try:
    with sync_playwright() as p:
        r.page(p).close()   # starts the browser (r.b) and the server
        base = f"http://127.0.0.1:{r.port}/static/teoria/"
        for scheme in ("light", "dark"):
            q = r.b.new_page(viewport={"width": 1280, "height": 900}, color_scheme=scheme, reduced_motion="reduce")
            try:
                for pr in PAGES:
                    q.goto(base + pr + ".html"); attendi(q, "document.readyState === 'complete' && document.body.innerText.length > 100")
                    attendi(q, "document.documentElement.dataset.theme === %r" % scheme)
                    for what, fn in (("axe", axe_ok), ("accessible names", names), ("Tab and focus indicator", tabs), ("24 px targets", targets)):
                        step(f"{pr} ({scheme}): {what}", lambda fn=fn: fn(q))
            finally: q.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
