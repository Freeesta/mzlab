"""The full-screen / exit arrow is centred in its button, in both states and in both themes."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8854, wd="/tmp/wd_fs")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.evaluate("document.querySelector('#fs-btn').hidden = false")      # headless Chromium may have no Fullscreen API: the markup is what is measured
        off = """cls => { const b = document.querySelector('#fs-btn'); b.querySelector('.fs-in').hidden = cls !== '.fs-in'; b.querySelector('.fs-out').hidden = cls !== '.fs-out';
            const s = b.querySelector(cls + ' svg').getBoundingClientRect(), bb = b.getBoundingClientRect(); return [s.left + s.width / 2 - (bb.left + bb.width / 2), s.top + s.height / 2 - (bb.top + bb.height / 2)]; }"""
        for theme in ("light", "dark"):
            def one(theme=theme):
                pg.evaluate(f"document.documentElement.setAttribute('data-theme', '{theme}')"); pg.wait_for_timeout(200)
                for cls in (".fs-in", ".fs-out"):
                    dx, dy = pg.evaluate(off, cls); assert abs(dx) <= 0.5 and abs(dy) <= 0.5, (theme, cls, dx, dy)
            step(f"centred in both states, {theme} theme", one)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
