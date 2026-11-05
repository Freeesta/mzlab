"""«Sample table» of the side panel never moves the page: scrollY and the gap between the header and the work area are the same before the click, after it and after «Back to work!».
Before a session the button used to scrollIntoView the table under the drop area, which jumped the page by the few pixels of the table that were cut off."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np, dati_sintetici as ds
tmp = Path(tempfile.mkdtemp(prefix="tabscroll_"))
FILES = []
for i in range(11):
    p = tmp / f"P_{i:02d}.mzML"; ds.ms2(p, np.random.default_rng(i), 15); FILES.append(str(p))
POS = """() => ({ max: Math.round(document.documentElement.scrollHeight - innerHeight), y: Math.round(scrollY), gap: Math.round(document.querySelector('#v-workspace').getBoundingClientRect().top + scrollY - document.querySelector('header').getBoundingClientRect().bottom) })"""
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8893, wd="/tmp/wd_tabscroll")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#drop"); attendi(pg, "window.qqReady === true", 60000)
        def load(files, w=1400, h=800):
            pg.set_viewport_size({"width": w, "height": h}); pg.set_input_files("#pick", files); open_first(pg); ready(pg)
        def same(a, b, what, gap=True):
            if not gap: a, b = dict(a, gap=0), dict(b, gap=0)      # a narrow window closes the side sheet: the work area moves by design, the scroll must not
            a = dict(a, y=min(a["y"], b["max"])); a.pop("max"); b = {k: v for k, v in b.items() if k != "max"}      # a shorter page clamps the scroll: that is the browser, not a jump
            assert a == b, f"{what}: {a} -> {b}"
        def press(n=1):
            for _ in range(n): pg.click("#samptab"); ready(pg)
        def run_cycle(y=0):
            """table on and off, twice, then the landing and «Back to work!»: the position never changes"""
            if y: pg.evaluate(f"scrollTo(0,{y})"); ready(pg)
            b = pg.evaluate(POS)
            press(2); same(b, pg.evaluate(POS), "after two presses")
            press(); press(); same(b, pg.evaluate(POS), "after table on and off again")
            pg.click("#np-closeall"); ready(pg)
            pg.evaluate("scrollTo(0,%d)" % y); ready(pg); b = pg.evaluate(POS)
            press(); same(b, pg.evaluate(POS), "landing -> table"); b = pg.evaluate(POS)      # the table page is shorter and may clamp the scroll: the next step starts from where it is
            press(); same(b, pg.evaluate(POS), "table -> landing")
            if pg.is_visible("#backwork"): pg.click("#backwork"); ready(pg)
            same(b, pg.evaluate(POS), "after «Back to work!»")
        load(FILES)
        step("11 files: no jump (page at the top)", lambda: run_cycle(0))
        step("11 files: no jump (page already scrolled by 10 px)", lambda: run_cycle(10))
        def nosess():
            """before a session the table is under the drop area, cut off at the bottom of the window: the button leaves the page where it is"""
            pg.click("#np-closeall"); ready(pg)
            h = pg.evaluate("Math.round(document.querySelector('#wtable').getBoundingClientRect().bottom) - 10")
            pg.set_viewport_size({"width": 1400, "height": h}); ready(pg)
            pg.evaluate("S.sess = false; syncWork(true); scrollTo(0, 0)"); ready(pg)
            b = pg.evaluate(POS); press(2); same(b, pg.evaluate(POS), "no session, table partly in view")
        step("before a session: a table partly in view does not scroll", nosess)
        def far():
            """a table that is entirely below the window still scrolls into view"""
            pg.set_viewport_size({"width": 1400, "height": 300}); ready(pg)
            pg.evaluate("scrollTo(0, 0)"); press()
            assert pg.evaluate("scrollY") > 0 or pg.evaluate("document.querySelector('#wtable').getBoundingClientRect().top < innerHeight"), "table not brought into view"
        step("before a session: a table out of view is brought into view", far)
        def one():
            pg.reload(); attendi(pg, "window.qqReady === true", 60000)
            load(FILES[:1]); run_cycle(0)
        step("1 file: no jump", one)
        def narrow():
            pg.reload(); attendi(pg, "window.qqReady === true", 60000)
            load(FILES, 800, 700)
            if pg.is_hidden("#samptab"): pg.click("#sb-toggle"); ready(pg)
            b = pg.evaluate(POS); press(); same(b, pg.evaluate(POS), "narrow window, table on", False)
            if pg.is_hidden("#samptab"): pg.click("#sb-toggle"); ready(pg)
            press(); same(b, pg.evaluate(POS), "narrow window, table off", False)
        step("narrow window (< 900 px): no jump", narrow)
        def hidden_panel():
            pg.reload(); attendi(pg, "window.qqReady === true", 60000)
            load(FILES)
            if pg.get_attribute("#sb-toggle", "aria-pressed") == "true": pg.click("#sb-toggle"); ready(pg)
            b = pg.evaluate(POS); pg.evaluate("document.querySelector('#samptab').click()"); ready(pg); same(b, pg.evaluate(POS), "side panel hidden")
        step("side panel hidden: no jump", hidden_panel)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
