"""No flash of the wrong theme: while the stylesheets and scripts of a page are still being fetched (delayed on purpose) the first frames already have the
background of the theme (system dark / saved dark / saved light), in the app and in the Teoria, and across app -> Teoria -> chapter -> app."""
import sys, os, io, re; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import lib
real_wait = lib._real_wft      # the patched wait_for_timeout asks the page, which is held on purpose here
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
DARK, LIGHT_APP, LIGHT_TEORIA = (0x14, 0x17, 0x1b), (0xf7, 0xf8, 0xfa), (0xfb, 0xfb, 0xf9)
def near(a, b): return all(abs(x - y) <= 3 for x, y in zip(a, b))
def early_pixel(b, url, scheme, pref):
    """Opens url with its stylesheets and external scripts held back and returns the computed background of <html> read before any of them ran
    (headless Chromium paints no frame while they are pending, so the computed style is what the first frame will use)."""
    ctx = b.new_context(color_scheme=scheme, viewport={"width": 800, "height": 600}, reduced_motion="reduce")
    if pref: ctx.add_init_script("try{localStorage.setItem('mzlab.prefs',JSON.stringify({theme:'%s'}))}catch(e){}" % pref)
    pg = ctx.new_page(); held = []; free = []
    def hold(route):
        route.continue_() if free else held.append(route)
    for pat in ("**/*.css", "**/teoria/tema.js", "**/static/i18n.js", "**/static/telefono.js"): pg.route(pat, hold)
    pg.goto(url, wait_until="commit"); real_wait(pg, 300)
    assert held, "nothing was held back"
    c = pg.evaluate("getComputedStyle(document.documentElement).backgroundColor")
    px = tuple(int(x) for x in re.findall(r"\d+", c)[:3])
    free.append(1)
    for h in held: h.continue_()
    real_wait(pg, 300); ctx.close(); return px
r = Run(port=8970, wd="/tmp/wd_tl")
try:
    with sync_playwright() as p:
        r.page(p); b = r.b; root = f"http://127.0.0.1:{r.port}/"
        cases = [("app", root, LIGHT_APP), ("teoria index", root + "static/teoria/index.html", LIGHT_TEORIA), ("teoria chapter", root + "static/teoria/06-esi.html", LIGHT_TEORIA)]
        for nm, url, light in cases:
            step(nm + ": system dark, no saved choice -> dark at the first frame", lambda url=url: (lambda px: (_ for _ in ()).throw(AssertionError(px)) if not near(px, DARK) else None)(early_pixel(b, url, "dark", None)))
            step(nm + ": saved dark over a light system -> dark", lambda url=url: (lambda px: (_ for _ in ()).throw(AssertionError(px)) if not near(px, DARK) else None)(early_pixel(b, url, "light", "dark")))
            step(nm + ": saved light over a dark system -> light", lambda url=url, light=light: (lambda px: (_ for _ in ()).throw(AssertionError(px)) if not near(px, light) else None)(early_pixel(b, url, "dark", "light")))
        def walk():
            ctx = b.new_context(color_scheme="dark", viewport={"width": 1000, "height": 700}); ctx.add_init_script(
                "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}))}catch(e){}"
                ";(()=>{window.__bg=[];const s=()=>window.__bg.push(getComputedStyle(document.documentElement).backgroundColor);document.addEventListener('DOMContentLoaded',s);requestAnimationFrame(s)})()")
            pg = ctx.new_page()
            for url in (root + "static/teoria/index.html", root + "static/teoria/04-lc.html", root + "static/teoria/pratica.html", root):
                pg.goto(url); pg.wait_for_timeout(800)
                bgs = pg.evaluate("window.__bg"); assert bgs and all(x == "rgb(20, 23, 27)" for x in bgs), (url, bgs)
            ctx.close()
        step("app -> Teoria -> chapter -> Pratica -> app: the html background is dark at DOMContentLoaded and at the first frame", walk)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
