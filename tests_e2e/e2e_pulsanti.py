"""A6: an active (.on) icon button is filled up to its edge: the pixels inside its corners and on its right side have the accent colour (light and dark theme, 200% zoom)."""
import sys, os, io; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from PIL import Image
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_pulsanti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8842, wd="/tmp/wd_pul")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        for dsf, theme in ((1, "light"), (1, "dark"), (2, "light")):
            ctx = r.b.new_context(viewport={"width": 1400, "height": 900}, device_scale_factor=dsf)
            pg = ctx.new_page(); pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
            if theme == "dark": pg.evaluate("document.documentElement.setAttribute('data-theme','dark')")
            pg.wait_for_timeout(2500)
            if pg.is_visible("#opbtn") or pg.is_visible("#pick"):
                pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); pg.wait_for_timeout(1000)
                pg.click("text=Carica dati"); ready(pg)
            pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(800)
            if theme == "dark": pg.evaluate("document.documentElement.setAttribute('data-theme','dark')"); pg.wait_for_timeout(300)
            def check():
                chrom = pg.locator(".pnl.chrom").first; chrom.scroll_into_view_if_needed()
                for a in ("iauto", "tlink"): 
                    b = chrom.locator(f'[data-a="{a}"]')
                    if "on" not in (b.get_attribute("class") or "").split(): b.click(); pg.wait_for_timeout(200)
                pg.mouse.move(5, 5)
                acc = pg.evaluate("(() => { const c = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim(); const d = document.createElement('i'); d.style.color = c; document.body.appendChild(d); const v = getComputedStyle(d).color.match(/\\d+/g).slice(0, 3).map(Number); d.remove(); return v; })()")
                btns = pg.locator(".pnl .bt.on")
                n = btns.count(); assert n >= 2, n
                for i in range(n):
                    bx = btns.nth(i); png = bx.screenshot(); im = Image.open(io.BytesIO(png)).convert("RGB"); w, h = im.size; k = max(3, round(3 * dsf))
                    for (x, y) in ((k, k), (w - 1 - k, k), (k, h - 1 - k), (w - 1 - k, h - 1 - k), (w - 1 - k, h // 2)):
                        px = im.getpixel((x, y))
                        if (x, y) in ((w - 1 - k, h // 2), (k, h // 2)): pass
                        assert max(abs(px[j] - acc[j]) for j in range(3)) <= 12 or sum(px) < 3 * 255 and False, (bx.get_attribute("data-a"), (x, y), px, acc, theme, dsf)
            step(f"active buttons filled to the edge ({theme}, {dsf}x)", check)
            ctx.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
