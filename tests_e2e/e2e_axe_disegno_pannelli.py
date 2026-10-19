"""axe-core color-contrast on the «Show panels» / «Hide panels» pill of Disegno in the dark theme (and the high-contrast dark one): the pill keeps its own opaque background.
axe comes from axe-playwright-python or $AXE_JS; without either the test says SKIP and passes."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

AXE = axe_path()
if not AXE:
    print("SKIP axe-core not available (pip install axe-playwright-python)"); sys.exit(0)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
r = Run(port=8933, wd="/tmp/wd_axe_dpan")
CC = "axe.run('#side-toggle', {runOnly: {type: 'rule', values: ['color-contrast']}}).then(r => r.violations.map(v => v.nodes.map(n => (n.any[0] || {}).message)))"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        attendi(pg, "window.TPDraw && TPDraw.ready()", 60000)
        pg.add_script_tag(path=AXE)
        def contrast(theme, extra=None):
            pg.evaluate("t => { document.documentElement.dataset.theme = t[0]; if (t[1]) document.documentElement.dataset.a11yBg = t[1]; else delete document.documentElement.dataset.a11yBg; }", [theme, extra])
            pg.mouse.move(2, 2); frames(pg)                          # at rest: the hover background must not be what makes it readable
            v = pg.evaluate(CC); assert not v, v
            ratio = pg.evaluate("""() => { const b = document.querySelector('#side-toggle'), c = getComputedStyle(b);
              return [c.backgroundColor, c.color, c.borderTopColor] }""")
            assert ratio[0] != "rgba(0, 0, 0, 0)", ("the pill needs its own background", ratio)
        for hidden in (False, True):
            name = "panels hidden («Show panels»)" if hidden else "panels shown («Hide panels»)"
            if hidden and not pg.evaluate("document.querySelector('#v-draw').classList.contains('noside')"): pg.click("#side-toggle"); attendi(pg, "document.querySelector('#v-draw').classList.contains('noside')")
            step(f"dark theme, {name}: text contrast", lambda: contrast("dark"))
            step(f"high-contrast dark, {name}: text contrast", lambda: contrast("dark", "hc"))
            step(f"light theme, {name}: text contrast", lambda: contrast("light"))
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:400]))
finally:
    r.close()
for s in steps: print(s)
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
