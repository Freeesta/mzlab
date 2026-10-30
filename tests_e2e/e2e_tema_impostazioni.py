"""Settings panel: clicking the three theme buttons in quick turns (while the soft view transition runs) keeps the panel open."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
r = Run(port=8897, wd="/tmp/wd97")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.emulate_media(reduced_motion="no-preference")          # the soft fade only runs with motion (Run starts with reduced motion; the page marked itself at load)
        pg.evaluate("document.documentElement.removeAttribute('data-a11y-reduce')")
        pg.click("#np-set"); attendi(pg, "!!document.querySelector('input[name=uip-th]')")
        assert pg.evaluate("typeof document.startViewTransition === 'function' && !document.documentElement.hasAttribute('data-a11y-reduce') && !document.documentElement.classList.contains('no-tr')"), "the fade is off in this browser"
        boxes = [pg.locator("input[name=uip-th]").nth(i).locator("xpath=..").bounding_box() for i in range(3)]
        for i in range(6):          # raw mouse clicks: Playwright's own click waits for the view transition to end, a hand does not
            b = boxes[i % 3]; pg.mouse.click(b["x"] + b["width"] / 2, b["y"] + b["height"] / 2)
        pg.wait_for_timeout(150)
        assert pg.evaluate("!!document.querySelector('input[name=uip-th]')"), "the settings panel closed during the theme fade"
        steps.append(("6 quick clicks on the theme buttons keep the panel open", "ok"))
finally:
    r.close(); r.report()
for s in steps: print(" ", s)
