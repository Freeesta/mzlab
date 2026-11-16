"""One-page Dati: the drop area and the sample table are ONE page. «Sample table» (#samptab) is always visible when there are rows and never a toggle: from the panels it opens
the page and scrolls the table under the header with the focus on its heading; on the page it only scrolls (smooth) and focuses, with no re-render, no layout shift and the panels
untouched. «Back to work!» gives back the panels and the scroll they had, with no jump. Checked at 1000, 1366 and 1600 px, with motion and with reduced motion."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np, dati_sintetici as ds
tmp = Path(tempfile.mkdtemp(prefix="tabscroll_"))
FILES = []
for i in range(11):
    p = tmp / f"P_{i:02d}.mzML"; ds.ms2(p, np.random.default_rng(i), 15); FILES.append(str(p))
POS = "() => ({ y: Math.round(scrollY), max: Math.round(document.documentElement.scrollHeight - innerHeight) })"
TOP = "() => Math.round(document.querySelector('#wtable').getBoundingClientRect().top - document.querySelector('header').getBoundingClientRect().bottom)"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + " | ".join(str(e).split("\n")[:6])[:600]))
def settle(pg):
    """the smooth scroll ends: scrollY is the same on three reads in a row"""
    last, n = None, 0
    for _ in range(80):
        y = pg.evaluate("scrollY"); n = n + 1 if y == last else 0; last = y
        if n >= 3: return y
        frames(pg, 2)
    raise AssertionError("the scroll never settled")
r = Run(port=8893, wd="/tmp/wd_tabscroll")
try:
    with sync_playwright() as p:
        pg = r.page(p, motion=True)
        pg.goto(f"http://127.0.0.1:{r.port}/")
        pg.wait_for_selector("#drop"); wait_for_js(pg, "window.qqReady === true", 60000)
        pg.set_viewport_size({"width": 1366, "height": 800}); pg.set_input_files("#pick", FILES); open_first(pg); ready(pg)
        def show_button():
            """the side panel follows the window width with a delay: the button counts when it is visible on two reads ten frames apart"""
            for _ in range(8):
                if pg.is_visible("#samptab"):
                    frames(pg, 10)
                    if pg.is_visible("#samptab"): return
                else: pg.click("#sb-toggle"); ready(pg)
            raise AssertionError(pg.evaluate("[innerWidth, document.getElementById('v-workspace').dataset.work, document.getElementById('samptab').hidden]"))
        def cycle(w):
            pg.set_viewport_size({"width": w, "height": 800}); ready(pg)
            show_button()      # a narrow window keeps the side panel closed
            assert pg.is_visible("#samptab"), "the button is visible"
            assert pg.get_attribute("#samptab", "aria-pressed") is None
            ids = pg.evaluate("E.panels.map(p => p.id).join()")
            pg.evaluate("scrollTo(0, 30)"); ready(pg); y0 = pg.evaluate("scrollY")
            show_button()      # the scroll may have closed the sheet of a narrow window
            pg.click("#samptab"); y1 = settle(pg)
            assert pg.evaluate("document.getElementById('v-workspace').dataset.work") == "add", "the one page"
            assert pg.is_visible("#start") and pg.is_visible("#wtable"), "drop area and table on the same page"
            top = pg.evaluate(TOP); assert top < 400, f"the table is brought under the header: {top}"
            assert pg.evaluate("document.activeElement.closest('#wtable') !== null"), "focus on the table heading"
            assert pg.is_visible("#samptab"), "the button stays"
            pg.click("#backwork"); ready(pg)
            assert pg.evaluate("document.getElementById('v-workspace').dataset.work") == "panels" and pg.evaluate("E.panels.map(p => p.id).join()") == ids, "panels as they were"
            y2 = pg.evaluate("scrollY"); assert abs(y2 - y0) <= 2, f"«Back to work!» restores the scroll: {y0} -> {y2}"
        for w in (1000, 1366, 1600): step(f"{w} px: panels -> one page -> back, the scroll comes back", lambda w=w: cycle(w))
        def inplace():
            pg.click("#samptab"); settle(pg); pg.evaluate("scrollTo(0, 0)"); ready(pg)
            pg.evaluate("""() => { window.__cls = []; new PerformanceObserver(l => l.getEntries().forEach(e => window.__cls.push(e.value))).observe({ type: 'layout-shift', buffered: false });
                document.querySelector('#wtable tbody tr').__mark = 1; window.__pn = document.querySelector('#dpanels').innerHTML.length; }""")
            pg.click("#samptab"); settle(pg); frames(pg, 6)
            assert pg.evaluate("document.querySelector('#wtable tbody tr').__mark === 1"), "the table was not re-rendered"
            assert pg.evaluate("document.getElementById('v-workspace').dataset.work") == "add", "no view switch"
            assert pg.evaluate("document.querySelector('#dpanels').innerHTML.length === window.__pn"), "panels untouched"
            assert pg.evaluate("window.__cls.filter(v => v > 0).length") == 0, pg.evaluate("window.__cls")
            assert pg.evaluate("document.activeElement.closest('#wtable') !== null")
        step("on the page: the button only scrolls (no re-render, no layout shift, panels unchanged)", inplace)
        def keyboard():
            pg.evaluate("scrollTo(0, 0)"); pg.focus("#samptab"); pg.keyboard.press("Enter"); settle(pg)
            assert pg.evaluate("document.activeElement.closest('#wtable') !== null"), "Enter reaches the table"
            pg.keyboard.press("Escape"); ready(pg)
            assert pg.evaluate("document.getElementById('v-workspace').dataset.work") == "panels", "Esc leaves the page"
        step("keyboard: Enter on the button, Esc back to the work", keyboard)
        def reduced():
            pg.emulate_media(reduced_motion="reduce"); pg.evaluate("scrollTo(0, 30)")
            show_button()
            pg.click("#samptab"); pg.wait_for_selector("#wtable", state="visible"); frames(pg, 3)
            assert pg.evaluate(TOP) < 400, "reduced motion: scrolled at once"
        step("reduced motion: the scroll is instant", reduced)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
