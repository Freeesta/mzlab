"""The leaf of the empty file list (#fl-hint): one line drawing drawn horizontal, tinted from the accent token, inside its box in both themes; still under reduced motion (the test page), gliding down once otherwise."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_foglia.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8982, wd="/tmp/wd_foglia")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate("document.getElementById('fl-hint').hidden = false")
        S = "#fl-hint svg.leaf"
        def shape():
            assert pg.locator(S).count() == 1 and pg.locator("svg.leaf").count() == 1, "one leaf, one definition"
            v = pg.evaluate(f"""(() => {{ const s = document.querySelector('{S}'), sw = parseFloat(getComputedStyle(s.querySelector('path:not(.lf)')).strokeWidth), r = s.getBoundingClientRect(), b = s.querySelector('.lf').getBoundingClientRect();
              return {{w: r.width, h: r.height, hid: s.getAttribute('aria-hidden'), sw, paths: s.querySelectorAll('path').length, inside: [b.left - r.left, b.top - r.top, r.right - b.right, r.bottom - b.bottom]}}; }})()""")
            assert v["w"] == 72 and v["h"] == 44 and v["hid"] == "true" and v["sw"] == 1.5, v
            assert all(x >= 0 for x in v["inside"]), ("the leaf does not fit its box", v)
            assert v["w"] > 1.4 * v["h"], ("the leaf is not drawn horizontal", v)
        step("one 72x44 svg, aria-hidden, 1.5 px stroke, drawn horizontal and inside its box", shape)
        def still():
            v = pg.evaluate(f"""[...document.querySelectorAll('{S}, {S} *')].map(e => {{ const c = getComputedStyle(e); return [e.tagName, c.animationName, c.transitionDuration, e.getAnimations().length]; }})""")
            assert all(a[1] == "none" and set(a[2].split(", ")) <= {"0s"} and a[3] == 0 for a in v), v
            assert pg.evaluate(f"document.querySelectorAll('{S} animate, {S} animateTransform, {S} set').length") == 0
        step("still under reduced motion: no animation, no transition", still)
        def tokens():
            for th in ("light", "dark"):
                pg.evaluate(f"document.documentElement.setAttribute('data-theme', '{th}')"); frames(pg, 2)
                c = pg.evaluate(f"""(() => {{ const s = document.querySelector('{S}'), f = getComputedStyle(s.querySelector('.lf')); return [getComputedStyle(s).color, f.fill, f.fillOpacity, getComputedStyle(document.documentElement).getPropertyValue('--accent').trim()]; }})()""")
                assert c[2] and 0 < float(c[2]) <= .15 and c[1] != "none", (th, c)
        step("a faint tint of the accent token and the muted line, in the light and the dark theme", tokens)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
