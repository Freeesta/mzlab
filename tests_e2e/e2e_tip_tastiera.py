"""Keyboard focus shows the same tooltip as the pointer (WCAG 1.4.13): Tab onto a control with a tip brings up #qtip with its text after the rest delay,
clear of the control, and the control keeps its accessible name; leaving the control hides it; Esc hides only the tooltip and leaves the Settings panel open (a second Esc closes the panel); a click does not."""
# covers: mzlab/web/tooltip.js
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tip_tastiera.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
# the tip of the focused control (the nearest title / data-tip, as tooltip.js looks it up), and whether #qtip shows it without covering the control
FOCUS = """() => { const e = document.activeElement, t = e && e.closest('[title],[data-tip],[data-tiph]');
  return { id: e ? e.id || e.tagName : '', tip: t ? (t.getAttribute('title') || t.dataset.tip || '').trim() : '' }; }"""
SHOWN = """want => { const q = document.getElementById('qtip'); if (q.hidden || q.textContent !== want) return false;
  const a = q.getBoundingClientRect(), b = document.activeElement.getBoundingClientRect();
  return a.right <= b.left || a.left >= b.right || a.bottom <= b.top || a.top >= b.bottom; }"""
def shown(pg, want):
    wait_for_js(pg, f"({SHOWN})({json.dumps(want)})", 8000)
def tab_to_tip(pg, root, n=30):
    for _ in range(n):
        pg.keyboard.press("Tab"); f = pg.evaluate(FOCUS)
        if f["tip"] and pg.evaluate(f"!!document.activeElement.closest('{root}')"): return f
    raise AssertionError(f"no control with a tip reached by Tab inside {root}")
r = Run(port=free_port(), wd="/tmp/wd_tiptast")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        wait_for_js(pg, "window.qqReady && !document.querySelector('#start').hidden")
        def settings_focus():
            pg.click("#np-set"); pg.wait_for_selector("#uipset"); pg.focus("#uip-x")
            f = tab_to_tip(pg, "#uipset")
            shown(pg, f["tip"])
            pg.keyboard.press("Tab"); f2 = pg.evaluate(FOCUS)
            if f2["tip"] != f["tip"]: wait_for_js(pg, f"(() => {{ const q = document.getElementById('qtip'); return q.hidden || q.textContent === {f2['tip']!r}; }})()", 4000)
        step("Settings: Tab onto a control shows its own tooltip, clear of it; Tab onward takes it away", settings_focus)
        def escape():
            f = tab_to_tip(pg, "#uipset"); shown(pg, f["tip"])
            pg.keyboard.press("Escape")
            assert pg.evaluate("document.getElementById('qtip').hidden"), "Esc hides the tooltip"
            assert pg.is_visible("#uipset") and pg.evaluate(FOCUS)["id"] == f["id"], "the panel stays open and the focus stays where it was"
            pg.keyboard.press("Escape"); wait_for_js(pg, "!document.querySelector('#uipset') || !document.querySelector('#uipset').offsetParent", 4000)
        step("Esc hides only the tooltip; a second Esc closes the panel", escape)
        def main_page():
            pg.evaluate("document.activeElement && document.activeElement.blur()"); f = tab_to_tip(pg, "body", 40)
            shown(pg, f["tip"])
            # the keyboard path leaves title in place: it is the accessible name of the icon-only buttons (axe button-name)
            assert pg.evaluate("(() => { const e = document.activeElement; return !!(e.getAttribute('aria-label') || e.getAttribute('title') || e.textContent.trim()); })()"), "the focused control keeps its accessible name"
        step("main page: the first control with a tip reached by Tab shows it", main_page)
        def click_no_tip():
            pg.keyboard.press("Escape"); pg.click("#np-set"); pg.wait_for_selector("#uipset"); pg.mouse.move(2, 2)
            t0 = pg.evaluate("(document.getElementById('uip-x').focus(), performance.now())")      # focus without a key, as a click leaves it
            wait_for_js(pg, f"performance.now() - {t0} > TIP_DELAY + 400", 8000)
            assert pg.evaluate("document.getElementById('qtip').hidden"), "a focus not made by the keyboard shows no tooltip"
        step("a focus that does not come from the keyboard shows no tooltip", click_no_tip)
finally:
    r.close()
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
sys.exit(1 if any(s != "ok" for _, s in steps) or len(steps) < 4 else 0)
