"""Logo easter egg (logo-piccone.js): 5 hits (click or Enter) switch mzFinder, the logo turns gold and stays gold after a reload;
the cracks close if the hits stop; with and without reduced motion.
Run from the repo root: python3 tests_e2e/e2e_logo_piccone.py"""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import *  # noqa: F401,F403
from playwright.sync_api import sync_playwright

R = Run(port=8863, wd="/tmp/wd_logo")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:400]))
LOGO = "header img.logo"
def cracks(pg): return pg.evaluate("document.querySelectorAll('#lg-fx .lg-cr.on').length")
def gold(pg): return pg.evaluate("[document.documentElement.hasAttribute('data-logo-gold'), localStorage.getItem('qqq.logo.gold'), getComputedStyle(document.querySelector('header img.logo')).filter]")
def tab(pg): return pg.evaluate("localStorage.getItem('qqq.mzfinder') === '1'")      # the tab itself exists only in the built site (e2e_tpmine*)
def run(p, reduced):
    tag = "reduced " if reduced else "motion "
    b = p.chromium.launch(); pg = b.new_page(viewport={"width": 1500, "height": 900}, reduced_motion="reduce" if reduced else "no-preference")
    pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
    pg.on("pageerror", lambda e: R.errs.append(("pageerror", str(e))))
    pg.goto(f"http://127.0.0.1:{R.port}/"); pg.wait_for_selector("#drop", timeout=60000); pg.wait_for_timeout(1500)
    def a11y():
        assert pg.get_attribute(LOGO, "tabindex") == "0" and pg.get_attribute(LOGO, "role") == "button"
        assert pg.get_attribute(LOGO, "alt") and pg.get_attribute(LOGO, "title") and gold(pg)[0] is False
    step(tag + "logo reachable by keyboard, label kept, not gold", a11y)
    def heal():
        pg.click(LOGO); pg.click(LOGO)
        assert cracks(pg) == 2, cracks(pg)
        pg.wait_for_function("document.querySelectorAll('#lg-fx .lg-cr.on').length === 0", timeout=6000)      # the real 3 s (wait_for_timeout of lib.py is shortened)
        assert cracks(pg) == 0 and not tab(pg)
    step(tag + "2 hits = 2 stages of cracks, they close after 3 s", heal)
    def five():
        for _ in range(5): pg.click(LOGO)
        pg.wait_for_function("localStorage.getItem('qqq.mzfinder') === '1'", timeout=15000)
        pg.wait_for_function("document.documentElement.hasAttribute('data-logo-gold')", timeout=8000)
        pg.wait_for_function("getComputedStyle(document.querySelector('header img.logo')).opacity === '1'", timeout=5000)
        g = gold(pg); assert g[1] == "1" and g[2] != "none", g
    step(tag + "5 clicks: mzFinder on, logo gold", five)
    def reload():
        pg.reload(); pg.wait_for_selector("#drop", timeout=60000); pg.wait_for_timeout(500)
        g = gold(pg); assert g[0] and g[2] != "none", g
    step(tag + "gold after reload", reload)
    def keyboard():
        pg.focus(LOGO)
        for _ in range(5): pg.keyboard.press("Enter"); pg.wait_for_timeout(80)
        pg.wait_for_function("localStorage.getItem('qqq.mzfinder') !== '1'", timeout=15000)
    step(tag + "Enter x5 switches mzFinder off", keyboard)
    b.close()
try:
    with sync_playwright() as p:
        run(p, True); run(p, False)
finally:
    R.close()
for n, s in steps: print(("OK  " if s == "ok" else "FAIL"), n, "" if s == "ok" else s)
bad = [s for s in steps if s[1] != "ok"]
errs = [e for e in R.errs if e[0] == "pageerror"]
if errs: print("page errors:", errs); bad.append(("errors", "x"))
sys.exit(1 if bad else 0)
