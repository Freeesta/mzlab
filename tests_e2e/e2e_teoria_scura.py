"""The Teoria follows the theme of the app (Chiaro / Scuro / Come il sistema): dark variables, SVG figures and canvas drawings readable, a change while the Teoria is open."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_teoria_scura.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
LUM = "(c=>{const m=c.match(/[0-9.]+/g).map(Number),f=v=>{v/=255;return v<=.03928?v/12.92:Math.pow((v+.055)/1.055,2.4)};return .2126*f(m[0])+.7152*f(m[1])+.0722*f(m[2])})"
def contrast(pg, js_a, js_b):
    return pg.evaluate(f"(()=>{{const L={LUM};const a=L({js_a}),b=L({js_b});return (Math.max(a,b)+.05)/(Math.min(a,b)+.05)}})()")
r = Run(port=8960, wd="/tmp/wd_ts")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        base = f"http://127.0.0.1:{r.port}/static/teoria/"
        def tf():
            return next(x for x in pg.frames if "/teoria/" in x.url)
        def open_dark():
            pg.click("#np-set"); pg.wait_for_timeout(200); pg.select_option("#uip-th", "dark"); pg.wait_for_timeout(300); pg.keyboard.press("Escape"); pg.mouse.click(5, 5)
            pg.click("#nav button[data-v=theory]"); pg.wait_for_timeout(1500)
            f = tf()
            assert f.evaluate("document.documentElement.dataset.theme") == "dark"
            c = f.evaluate("getComputedStyle(document.body).backgroundColor"); print("bg", c)
            assert contrast(f, "getComputedStyle(document.body).color", "getComputedStyle(document.body).backgroundColor") > 7
        step("dark theme of the app: the Teoria is dark with readable text", open_dark)
        def chapters():
            f = tf()
            for name, shot in (("06-esi.html", "esi"), ("04-lc.html", "lc"), ("05-gc.html", "gc")):
                f.goto(base + name); f.wait_for_timeout(1200)
                assert f.evaluate("document.documentElement.dataset.theme") == "dark", name
                assert contrast(f, "getComputedStyle(document.body).color", "getComputedStyle(document.body).backgroundColor") > 7
                f.evaluate("document.querySelector('svg,table')?.scrollIntoView({block:'center'})"); f.wait_for_timeout(300)
                pg.screenshot(path=SH + f"teoria_scuro_{shot}.png")
                # the figures: no text in the SVG is dark on a dark background
                bad = f.evaluate(f"""(()=>{{const L={LUM},bg=getComputedStyle(document.body).backgroundColor;const out=[];document.querySelectorAll('main svg text').forEach(t=>{{const c=getComputedStyle(t).fill;if(!c||c==='none'||c.indexOf('rgb')<0)return;const a=L(c),b=L(bg);if((Math.max(a,b)+.05)/(Math.min(a,b)+.05)<3)out.push(c)}});return out}})()""")
                assert not bad, (name, bad[:3])
        step("three chapters (text, SVG figure, table): dark and readable", chapters)
        def sim():
            f = tf(); f.goto(base + "06-esi.html"); f.wait_for_timeout(1500)
            n = f.evaluate("document.querySelectorAll('.sim canvas').length"); assert n >= 1, n
            dark = f.evaluate("(()=>{const c=document.querySelector('.sim canvas');const g=c.getContext('2d');const d=g.getImageData(Math.floor(c.width/2),Math.floor(c.height/2),1,1).data;return d[0]+d[1]+d[2]})()")
            assert dark < 400, dark                                                      # the middle of the canvas is not the white of the light theme
        step("an interactive drawing is dark too", sim)
        def live():
            pg.click("#stbar [data-tcmd=impostazioni]"); f = tf(); f.wait_for_selector("#a11y-panel:not([hidden])"); f.check("input[name=a11y-tema][value=light]"); pg.wait_for_timeout(2000)      # the gear of the Teoria: the panel has the theme
            f = tf(); assert f.evaluate("document.documentElement.dataset.theme") == "light"
            assert contrast(f, "getComputedStyle(document.body).color", "getComputedStyle(document.body).backgroundColor") > 7
            pg.screenshot(path=SH + "teoria_chiaro.png")
        step("changing to Chiaro while the Teoria is open switches it", live)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
