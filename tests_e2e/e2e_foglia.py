"""The leaf of the empty file list (#fl-hint): one still line drawing, tinted from the accent token, inside its box, never animated, in both themes."""
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
            v = pg.evaluate(f"""(() => {{ const s = document.querySelector('{S}'), g = s.querySelector('g'), b = g.getBBox(), sw = parseFloat(getComputedStyle(s.querySelector('path:not(.lf)')).strokeWidth);
              return {{w: s.getBoundingClientRect().width, h: s.getBoundingClientRect().height, hid: s.getAttribute('aria-hidden'), b: [b.x, b.y, b.x + b.width, b.y + b.height], sw, paths: s.querySelectorAll('path').length}}; }})()""")
            assert 64 <= v["w"] <= 80 and v["w"] == v["h"] and v["hid"] == "true" and v["sw"] == 1.5, v
            m = v["sw"] / 2 + .5      # the stroke reaches half its width past the geometry
            assert v["b"][0] >= m and v["b"][1] >= m and v["b"][2] <= 80 - m and v["b"][3] <= 80 - m, ("the leaf does not fit its box", v)
        step("one svg of 64-80 px, aria-hidden, 1.5 px stroke, the whole leaf inside its box", shape)
        def still():
            v = pg.evaluate(f"""[...document.querySelectorAll('{S}, {S} *')].map(e => {{ const c = getComputedStyle(e); return [e.tagName, c.animationName, c.transitionDuration, e.getAnimations().length]; }})""")
            assert all(a[1] == "none" and set(a[2].split(", ")) <= {"0s"} and a[3] == 0 for a in v), v
            assert pg.evaluate(f"document.querySelectorAll('{S} animate, {S} animateTransform, {S} set').length") == 0
        step("no animation and no transition on the leaf or inside it", still)
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
