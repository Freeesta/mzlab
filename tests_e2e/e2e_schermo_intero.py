# e2e_schermo_intero: full screen button (icon, title, aria-pressed follow fullscreenchange) and the safety margin above the header
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8839, wd="/tmp/wd_fs")
ST = "(()=>{const b=document.querySelector('#fs-btn');return {fs:!!document.fullscreenElement,pr:b.getAttribute('aria-pressed'),t:b.title,al:b.getAttribute('aria-label'),inn:b.querySelector('.fs-in').hidden,out:b.querySelector('.fs-out').hidden,top:document.querySelector('header').getBoundingClientRect().height,pad:getComputedStyle(document.querySelector('header')).paddingTop}})()"
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.wait_for_timeout(500)
        h = {}
        def normal():
            s = pg.evaluate(ST); h.update(s)
            assert s["pr"] == "false" and s["t"] == "Schermo intero" and s["al"] == "Schermo intero" and not s["inn"] and s["out"] and s["pad"] == "4px", s
        step("window: expand icon, title «Schermo intero», no margin", normal)
        def full():
            pg.click("#fs-btn"); pg.wait_for_function("!!document.fullscreenElement"); pg.wait_for_timeout(300)
            s = pg.evaluate(ST)
            assert s["pr"] == "true" and "Esc" in s["t"] and s["al"] == s["t"] and s["inn"] and not s["out"], s
            assert s["pad"] == "18px" and abs(s["top"] - h["top"] - 14) < 1.5, (s, h)
        step("full screen: collapse icon, title with Esc, 14 px margin only on top", full)
        def back():
            pg.evaluate("document.exitFullscreen()"); pg.wait_for_function("!document.fullscreenElement"); pg.wait_for_timeout(300)
            s = pg.evaluate(ST); assert s["pr"] == "false" and s["t"] == "Schermo intero" and s["pad"] == "4px" and not s["inn"], s
        step("leaving (as Esc does) restores icon, title and margin", back)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
