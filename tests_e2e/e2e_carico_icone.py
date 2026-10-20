"""Loading screen: the method field is a slim row (no card box, hidden step heading, still a drop target and a file picker); the loading overlay and the shared
progress panel show a turning ring (still with reduced motion) and the finished panel a check mark."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
HINTS = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1')}catch(_){}"
r = Run(port=8972, wd="/tmp/wd_ci")
def opened(b, motion):
    c = b.new_context(viewport={"width": 1280, "height": 800}, reduced_motion=motion)
    c.add_init_script(HINTS)
    pg = c.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/?lang=en"); pg.wait_for_selector("#drop", timeout=120000)
    return c, pg
ANIM = "sel => getComputedStyle(document.querySelector(sel)).animationName"
PANEL = """async () => { const m = await import('./static/avanzamento.js'); const a = m.avanza(document.querySelector('#start'), {title: 'Job', n: 1});
    window.__panel = a; const b = document.querySelector('.avanza .cp-head > b'); return getComputedStyle(b, '::before').animationName; }"""
try:
    with sync_playwright() as p:
        r.page(p); b = r.b
        def row():
            c, pg = opened(b, "no-preference")
            bb = pg.locator("#dropdam").bounding_box(); assert bb["height"] < 44, bb
            assert pg.evaluate("getComputedStyle(document.querySelector('.method-row')).borderTopWidth") == "0px"
            t = pg.inner_text("#start"); assert "Add acquisition method" in t and "optional" in t and "2. Acquisition method" in t, t
            assert pg.evaluate("document.querySelector('.method-row h3').getBoundingClientRect().height") <= 1
            with pg.expect_file_chooser(timeout=10000): pg.click("#dropdam")
            c.close()
        step("method field: slim row, still opens the file picker", row)
        def motion():
            c, pg = opened(b, "no-preference")
            pg.evaluate("loadingStato('Reading the files', 40)")
            pg.wait_for_selector("#ldmsg .ldic", state="attached", timeout=10000)
            assert pg.evaluate(ANIM, "#ldmsg .ldic") == "cpspin"
            assert pg.evaluate(PANEL) == "cpspin"
            pg.evaluate("window.__panel.ok(); window.__panel.end()")
            pg.wait_for_selector(".avanza .cp-done", timeout=10000)
            assert pg.evaluate("getComputedStyle(document.querySelector('.avanza .cp-done > b'), '::before').content") == '"✓"'
            c.close()
        step("motion allowed: turning ring in the overlay and the panel, check mark when done", motion)
        def still():
            c, pg = opened(b, "reduce")
            pg.evaluate("loadingStato('Reading the files', 40)")
            pg.wait_for_selector("#ldmsg .ldic", state="attached", timeout=10000)
            assert pg.evaluate(ANIM, "#ldmsg .ldic") == "none"
            assert pg.evaluate(PANEL) == "none"
            c.close()
        step("reduced motion: the rings do not turn", still)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
