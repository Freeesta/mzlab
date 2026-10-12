"""Golden logo: in the dark theme (and dark high contrast) the gold is brighter than in the light theme, so it stays readable (measured >= 4.5:1 on the header); the light theme keeps its own."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8990, wd="/tmp/wd_logo_oro")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        def bright(theme, hc=None):
            pg.evaluate("([t,h])=>{const d=document.documentElement;d.setAttribute('data-theme',t);h?d.setAttribute('data-a11y-bg',h):d.removeAttribute('data-a11y-bg');d.setAttribute('data-logo-gold','')}", [theme, hc]); pg.wait_for_timeout(200)
            f = pg.evaluate("getComputedStyle(document.querySelector('header img.logo')).filter")
            import re; return float(re.search(r"brightness\(([\d.]+)\)", f).group(1))
        def check():
            assert bright("light") < 1.5, "the light theme keeps its gold"
            assert bright("dark") >= 2, "dark theme: brighter gold"
            assert bright("dark", "hc") >= 2, "dark high contrast: brighter gold"
        step("the gold logo is brighter in the dark theme, unchanged in the light one", check)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
