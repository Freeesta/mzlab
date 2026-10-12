# e2e_x_centrata: the close «X» of the panels, of «close all» and of dialogs is centred in its button (centroid of the drawn glyph = centre of the box, ±0.6 px)
import os, sys, io; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
from PIL import Image
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8844, wd="/tmp/wd_xc")
def off(pg, sel):
    pg.mouse.move(2, 2); loc = pg.locator(sel).first; loc.scroll_into_view_if_needed(); b = loc.bounding_box()
    x0, y0 = int(b["x"]), int(b["y"]); w, h = int(round(b["x"] + b["width"])) - x0, int(round(b["y"] + b["height"])) - y0
    im = np.asarray(Image.open(io.BytesIO(pg.screenshot(clip={"x": x0, "y": y0, "width": w, "height": h}))).convert("RGB")).astype(int)
    bg = im[1, 1]; m = np.abs(im - bg).sum(axis=2) > 120
    inner = m.copy(); inner[:2, :] = inner[-2:, :] = False; inner[:, :2] = inner[:, -2:] = False      # the rounded border is not the glyph
    assert inner.sum() > 8, (sel, "no glyph found")
    ys, xs = np.nonzero(inner); gx, gy = xs.mean() + 0.5 + x0, ys.mean() + 0.5 + y0
    return gx - (b["x"] + b["width"] / 2), gy - (b["y"] + b["height"] / 2)
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(800); pg.click("text=Carica dati"); ready(pg)
        for name, sel in (("«close all»", "#np-closeall"), ("panel", ".pnl .hd .x")):
            def chk(sel=sel):
                dx, dy = off(pg, sel); assert abs(dx) <= 0.6 and abs(dy) <= 0.6, (sel, round(dx, 2), round(dy, 2), pg.evaluate("(()=>{const e=document.querySelector(%r);const c=getComputedStyle(e),b=e.getBoundingClientRect();return [b.width,b.height,c.display,c.padding,c.fontSize,c.alignItems,c.lineHeight,c.boxSizing]})()" % sel))
            step(name + " X centred in its button", chk)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
