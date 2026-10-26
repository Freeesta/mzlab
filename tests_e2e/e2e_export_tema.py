"""Saved PNG of the TIC (cascade included) and of a spectrum is the same in light and dark theme: white background, dark text, pale fill under the traces."""
import sys, os, io; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from PIL import Image, ImageChops
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8881, wd="/tmp/wd81")
def png(pg, scope):
    with pg.expect_download() as d: dlmenu(pg, scope, "png")
    return Image.open(io.BytesIO(open(d.value.path(), "rb").read())).convert("RGB")
def dark_frac(im):
    b = im.tobytes(); n = len(b) // 3; return sum(1 for i in range(0, len(b), 3) if b[i] + b[i + 1] + b[i + 2] < 150) / n
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_FullMass-t15")]); ready(pg)
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');p.mode='cas';draw(p)})()"); ready(pg); frames(pg)
        shots = {}
        for theme in ("light", "dark"):
            pg.evaluate(f"(()=>{{UIP.theme='{theme}';document.documentElement.dataset.theme='{theme}';paintFiles();redrawAll()}})()"); ready(pg); frames(pg)
            shots[theme] = (png(pg, ".pnl.chrom"), png(pg, ".pnl.spec"))
        for i, kind in enumerate(("TIC cascade", "spectrum")):
            def same(i=i):
                a, b = shots["light"][i], shots["dark"][i]
                assert a.size == b.size, (a.size, b.size)
                d = ImageChops.difference(a, b).getbbox(); assert d is None, ("light and dark PNG differ", d)
            step(f"{kind}: PNG identical in light and dark", same)
            def readable(i=i):
                im = shots["dark"][i]; assert dark_frac(im) < 0.2, "black area in the dark PNG"
                assert im.getpixel((2, im.size[1] - 2)) == (255, 255, 255), "background not white"
            step(f"{kind}: white background, no black areas (dark theme)", readable)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
