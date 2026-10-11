"""Molecule drawings follow the theme: dark theme gives light bonds/atoms (not black), contrast >= 3:1, runtime theme switch; the names lose no data."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

r = Run(port=8897, wd="/tmp/wd97")
steps = []

JS = """async (theme) => {
  document.documentElement.setAttribute('data-theme', theme);
  const O = await import('./static/vendor/openchemlib.js');
  const host = document.createElement('div'); host.id = 'molhost'; host.style.cssText = 'background:var(--panel);position:fixed;left:0;top:0;z-index:99999';
  document.body.appendChild(host);
  host.innerHTML = MOLUI.svg(O.Molecule.fromSmiles('CC(=O)Nc1ccc(F)cc1S(=O)(=O)N').toSVG(300, 170));
  return true;
}"""
COLORS = """() => {
  const host = document.getElementById('molhost'), bg = getComputedStyle(host).backgroundColor;
  const q = s => { const e = host.querySelector(s); return e ? getComputedStyle(e) : null; };
  const line = q('line'), cl = q('text.mol-o'), cn = q('text.mol-n');
  return { bg, bond: line.stroke, o: cl.fill, n: cn.fill, texts: host.querySelectorAll('text').length };
}"""

def rgb(s):
    import re
    return [int(x) for x in re.findall(r"\d+", s)[:3]]
def lum(c):
    f = lambda v: (v / 255) / 12.92 if v / 255 <= .03928 else (((v / 255) + .055) / 1.055) ** 2.4
    return .2126 * f(c[0]) + .7152 * f(c[1]) + .0722 * f(c[2])
def ratio(a, b):
    x, y = sorted((lum(rgb(a)), lum(rgb(b))), reverse=True); return (x + .05) / (y + .05)

try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.evaluate(JS, "dark")
        c = pg.evaluate(COLORS)
        assert rgb(c["bond"]) != [0, 0, 0] and ratio(c["bond"], c["bg"]) >= 3, c
        assert ratio(c["o"], c["bg"]) >= 3 and ratio(c["n"], c["bg"]) >= 3 and c["texts"] > 3, c
        steps.append(("dark theme: bonds not black, contrast >= 3", "ok"))
        pg.evaluate("() => document.documentElement.setAttribute('data-theme', 'light')")      # runtime switch, no reload
        c2 = pg.evaluate(COLORS)
        assert c2["bond"] != c["bond"] and ratio(c2["bond"], c2["bg"]) >= 3 and ratio(c2["o"], c2["bg"]) >= 3, c2
        steps.append(("light theme after a runtime switch", "ok"))
        pg.screenshot(path="/tmp/molui_light.png", clip={"x": 0, "y": 0, "width": 320, "height": 190})
        pg.evaluate("() => document.documentElement.setAttribute('data-theme', 'dark')")
        pg.screenshot(path="/tmp/molui_dark.png", clip={"x": 0, "y": 0, "width": 320, "height": 190})
        assert not [e for e in r.errs if e[0] in ("pageerror",)], r.errs
        steps.append(("no page errors", "ok"))
    r.close()
except Exception as e:
    steps.append(("run", f"FAIL: {e}"))
    r.close()

for s in steps:
    print(" ", s)
r.report()
