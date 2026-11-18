# e2e_pannelli_accanto: the «Hide panels» button (#sb-toggle) sits right next to the last visible tab of the header at every width (also when the header wraps), the header never overflows, and the Flow tab icon is point-symmetric when rendered at 2x
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8846, wd="/tmp/wd_pannelli_accanto")
JS = """(()=>{const t=[...document.querySelectorAll('#nav button')].filter(b=>{const q=b.getBoundingClientRect();return q.width&&getComputedStyle(b).visibility!=='hidden'});
const l=t[t.length-1].getBoundingClientRect(),s=document.querySelector('#sb-toggle').getBoundingClientRect();
return {tabs:t.length,gap:Math.round(s.left-l.right),dy:Math.round(s.top-l.top),w:Math.round(s.width)}})()"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        for w in (500, 600, 700, 900, 1024, 1200, 1440):
            def chk(w=w):
                pg.set_viewport_size({"width": w, "height": 900}); pg.reload()
                wait_for_js(pg, "document.querySelector('#stbar .tb') && !document.documentElement.classList.contains('i18n-wait')"); frames(pg, 5)
                d = pg.evaluate(JS)
                assert d["w"] > 0 and abs(d["dy"]) < 20 and 0 <= d["gap"] <= 12, (w, d)
            step(f"{w} px: the panels button follows the last tab within 12 px", chk)
        def fits():
            pg.set_viewport_size({"width": 390, "height": 800}); pg.reload()
            wait_for_js(pg, "document.querySelector('#stbar .tb') && !document.documentElement.classList.contains('i18n-wait')"); frames(pg, 5)
            h = pg.evaluate("(() => { const h = document.querySelector('header'); return [h.scrollWidth, h.clientWidth, document.documentElement.scrollWidth, innerWidth] })()")
            assert h[0] <= h[1] + 1 and h[2] <= h[3], h
        step("390 px: the header and the page do not overflow sideways", fits)
        def icon():
            # the Flow glyph rendered at 2x (64 px canvas) must equal itself rotated by 180 degrees, and sit centred in the 24 px viewBox
            d = pg.evaluate("""async () => {
                const svg = document.querySelector('#nav [data-v=flow] svg'); const x = new XMLSerializer().serializeToString(svg).replace('width="16" height="16"', 'width="64" height="64"').replace('currentColor', '#000');
                const img = new Image(); img.src = URL.createObjectURL(new Blob([x], {type: 'image/svg+xml'})); await img.decode();
                const c = document.createElement('canvas'); c.width = c.height = 64; const g = c.getContext('2d'); g.drawImage(img, 0, 0);
                const a = g.getImageData(0, 0, 64, 64).data; let diff = 0, bad = 0, ink = 0, minx = 64, maxx = 0, miny = 64, maxy = 0;
                for (let y = 0; y < 64; y++) for (let xx = 0; xx < 64; xx++) { const v = a[(y * 64 + xx) * 4 + 3], w = a[((63 - y) * 64 + (63 - xx)) * 4 + 3]; const e = Math.abs(v - w); diff = Math.max(diff, e); if (e > 8) bad++; if (v > 40) { ink++; minx = Math.min(minx, xx); maxx = Math.max(maxx, xx); miny = Math.min(miny, y); maxy = Math.max(maxy, y); } }
                return {diff, bad, ink, cx: minx + maxx, cy: miny + maxy};
            }""")
            # Skia anti-aliases the four rounded corners not exactly point-symmetrically (<= 56/255 on ~54 px); the old asymmetric glyph gave 255 on ~400 px
            assert d["ink"] > 100 and d["diff"] <= 64 and d["bad"] <= 80 and abs(d["cx"] - 63) <= 1 and abs(d["cy"] - 63) <= 1, d
        step("Flow tab icon: point-symmetric and centred at 2x", icon)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if any(s[1] != "ok" for s in steps) else 0)
