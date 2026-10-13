"""Guided tour of the Disegno view (TOUR.disegno(), chapter 3): 7 steps, each finds its visible target (the zoom buttons live inside the Ketcher iframe), the
arrow points at it, the steps about the right-hand panels show them if the student hid them and hide them again at the end, «Fine» and Escape close the tour
without touching the state of the Dati invitation (qqq.tour.lr), English texts."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
GEO = """(() => {
  const d = document.getElementById('tour-dialog'), s = document.getElementById('tour-spotlight'); if (!d || !s) return null;
  const a = d.querySelector('.tour-arrow'), ar = a.getBoundingClientRect(), sr = s.getBoundingClientRect(), side = d.className.replace('tour-side-', '');
  return {side, hd: d.querySelector('.tour-hd').textContent, ttl: d.querySelector('h3').textContent, ax: ar.left + ar.width / 2, ay: ar.top + ar.height / 2,
    hidden: getComputedStyle(a).display === 'none', l: sr.left, r: sr.right, t: sr.top, b: sr.bottom, w: sr.width, h: sr.height};
})()"""
r = Run(port=8993, wd="/tmp/wd_tour_disegno")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.click("#nav [data-v=draw]")
        pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.wait_for_timeout(1500)
        def walk():
            pg.evaluate("TOUR.disegno()"); pg.wait_for_timeout(500)
            seen, bad = [], []
            for _ in range(10):
                if not pg.evaluate("!!TOUR.active"): break
                g = pg.evaluate(GEO)
                assert g, "no dialog"
                seen.append(g["ttl"])
                assert g["hd"].endswith("Disegno") and g["w"] > 20 and g["h"] > 20, g
                if not g["hidden"]:
                    horiz = g["side"] in ("bottom", "top"); c, lo, hi = (g["ax"], g["l"], g["r"]) if horiz else (g["ay"], g["t"], g["b"])
                    if not (lo - 3 <= c <= hi + 3): bad.append((g["hd"], g["side"], round(c), round(lo), round(hi)))
                if pg.locator("#tour-btn-fine").count(): break
                pg.evaluate("TOUR.avanti()"); pg.wait_for_timeout(350)
            assert len(seen) == 7, seen                                  # no step skipped for a missing target
            assert not bad, bad
            assert pg.locator("#tour-btn-fine").count() == 1 and pg.locator("#tour-btn-back").count() == 1
        step("7 steps, each with its visible target and the arrow on it", walk)
        def close():
            pg.click("#tour-btn-fine"); pg.wait_for_timeout(300)
            assert not pg.evaluate("TOUR.active") and pg.locator("#tour-dialog").count() == 0
            assert pg.evaluate("localStorage.getItem('qqq.tour.lr')") is not None and "saltato" in pg.evaluate("localStorage.getItem('qqq.tour.lr')")   # untouched
        step("«Fine» closes it and leaves the Dati invitation state alone", close)
        def hidden_panels():
            pg.click("#side-toggle"); pg.wait_for_timeout(300)
            assert pg.evaluate("document.getElementById('v-draw').classList.contains('noside')")
            pg.evaluate("TOUR.inizia(3, 3)"); pg.wait_for_timeout(500)
            assert not pg.evaluate("document.getElementById('v-draw').classList.contains('noside')"), "panels not shown"
            assert pg.evaluate(GEO)["w"] > 20
            pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            assert not pg.evaluate("TOUR.active")
            assert pg.evaluate("document.getElementById('v-draw').classList.contains('noside')"), "panels not hidden again"
            pg.click("#side-toggle"); pg.wait_for_timeout(300)
        step("hidden panels are shown for their steps and hidden again; Escape closes", hidden_panels)
        def english():
            pg.goto(f"http://127.0.0.1:{r.port}/?lang=en#/disegno"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000); pg.wait_for_timeout(1500)
            pg.evaluate("TOUR.disegno()"); pg.wait_for_timeout(400)
            g = pg.evaluate(GEO); assert g["hd"].endswith("Drawing") and g["ttl"] == "The canvas", g
            pg.keyboard.press("Escape")
        step("English texts", english)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
