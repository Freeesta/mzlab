"""Safari and iPadOS refuse a canvas over 16.7 Mpx (toBlob gives null): the PNG exports that rasterise at a multiple of their size stay under it
(compat.js canvasScale, the global canvasScale of the classic scripts) and a notice gives the final size. The cap is stubbed on HTMLCanvasElement.toBlob, as
WebKit behaves: a wide drawing and a large MSn graph SVG export a PNG of at most 16 777 216 pixels; the size rule of the 3D map export on a 2x screen
(mappa3d.js pngScale) is checked without WebGL. No browser errors."""
import sys, os, struct; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_safari_canvas.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
MAX = 16777216
CAP = """(() => { const tb = HTMLCanvasElement.prototype.toBlob; window.__troppo = null;
HTMLCanvasElement.prototype.toBlob = function (cb, ...a) { if (this.width * this.height > %d) { window.__troppo = [this.width, this.height]; setTimeout(() => cb(null)); return; } return tb.call(this, cb, ...a); }; })()""" % MAX
# two atoms 100 x 90 units apart: Ketcher draws them about 2500 x 2250 px, which the old 3x export made a 50 Mpx canvas
MOL = "\\n  mzlab\\n\\n  2  0  0  0  0  0  0  0  0  0999 V2000\\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\\n  100.0000   90.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\\nM  END\\n"
TOASTS = "[...document.querySelectorAll('.toast-msg')].map(e => e.textContent)"
def dims(path):
    b = open(path, "rb").read(24); assert b[:8] == b"\x89PNG\r\n\x1a\n", b[:8]
    return struct.unpack(">II", b[16:24])
r = Run(port=8860, wd="/tmp/wd_safari_canvas")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.add_init_script(CAP); pg.reload(); ready(pg)
        def drawing():
            pg.evaluate("setView('draw')"); attendi(pg, "TPDraw.ready()", 60000)
            pg.evaluate(f"document.getElementById('kframe').contentWindow.ketcher.setMolecule('{MOL}')"); frames(pg, 30)
            with pg.expect_download(timeout=60000) as d: pg.click("#ex-png")
            w, h = dims(d.value.path())
            assert not pg.evaluate("window.__troppo"), f"a canvas over the cap: {pg.evaluate('window.__troppo')}"
            assert w * h <= MAX and w * h > MAX * 0.9, f"PNG {w} x {h}: not just under the cap"
            t = pg.evaluate(TOASTS); assert any(f"{w} × {h} px" in x for x in t), f"no notice of the size {w} × {h}: {t}"
        step("Draw: a wide drawing exports a PNG just under Safari's canvas cap", drawing)
        def graph():
            out = pg.evaluate("""async () => { const m = await import('/static/msngrafo-immagine.js');
              const b = await m.pngDa('<svg xmlns="http://www.w3.org/2000/svg" width="3000" height="2000"><rect width="3000" height="2000" fill="#123456"/></svg>', 2);
              const i = await createImageBitmap(b); return [i.width, i.height, window.__troppo]; }""")
            assert not out[2] and out[0] * out[1] <= MAX and out[0] > 4000, out
            t = pg.evaluate(TOASTS); assert any(f"{out[0]} × {out[1]} px" in x for x in t), f"no notice of the size: {t}"
        step("MSn graph: a large graph exports its PNG under the cap", graph)
        def map3d():
            # the size rule of the 3D export (mappa3d.js pngScale) without WebGL: a 1400 x 900 viewer on a 2x screen was a 5600 x 3600 canvas (20 Mpx)
            out = pg.evaluate("""async () => { const m = await import('/static/mappa3d.js'); const s = m.pngScale(1400, 900, 2), k = m.pngScale(600, 400, 2);
              return [s, Math.round(1400 * 2 * s), Math.round(900 * 2 * s), k]; }""")
            s, w, h, k = out
            assert s < 2 and w * h <= MAX and w * h > 0.99 * MAX and k == 3, out
            t = pg.evaluate(TOASTS); assert any(f"{w} × {h} px" in x for x in t), f"no notice of the size {w} × {h}: {t}"
        step("3D map on a 2x screen: the PNG stays under the cap", map3d)
finally:
    r.close()
r.report()
for s in steps: print("  ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
