"""Safari and iPadOS refuse a canvas over 16.7 Mpx (toBlob gives null): the PNG exports that rasterise at a multiple of their size stay under it
(compat.js canvasScale / scalaTela). The cap is stubbed on HTMLCanvasElement.toBlob, as WebKit behaves; a wide drawing, a large MSn graph SVG and the 3D map
on a 2x screen export a PNG of at most 16 777 216 pixels. No browser errors."""
import sys, os, struct; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tela_safari.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
MAX = 16777216
CAP = """(() => { const tb = HTMLCanvasElement.prototype.toBlob; window.__troppo = null;
HTMLCanvasElement.prototype.toBlob = function (cb, ...a) { if (this.width * this.height > %d) { window.__troppo = [this.width, this.height]; setTimeout(() => cb(null)); return; } return tb.call(this, cb, ...a); }; })()""" % MAX
# two atoms 100 x 90 units apart: Ketcher draws them about 2500 x 2250 px, which the old 3x export made a 50 Mpx canvas
MOL = "\\n  mzlab\\n\\n  2  0  0  0  0  0  0  0  0  0999 V2000\\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\\n  100.0000   90.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\\nM  END\\n"
def dims(path):
    b = open(path, "rb").read(24); assert b[:8] == b"\x89PNG\r\n\x1a\n", b[:8]
    return struct.unpack(">II", b[16:24])
r = Run(port=8860, wd="/tmp/wd_tela_safari")
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
        step("Draw: a wide drawing exports a PNG just under Safari's canvas cap", drawing)
        def graph():
            out = pg.evaluate("""async () => { const m = await import('/static/msngrafo-immagine.js');
              const b = await m.pngDa('<svg xmlns="http://www.w3.org/2000/svg" width="3000" height="2000"><rect width="3000" height="2000" fill="#123456"/></svg>', 2);
              const i = await createImageBitmap(b); return [i.width, i.height, window.__troppo]; }""")
            assert not out[2] and out[0] * out[1] <= MAX and out[0] > 4000, out
        step("MSn graph: a large graph exports its PNG under the cap", graph)
        def map3d():
            c = r.b.new_context(viewport={"width": 1440, "height": 950}, device_scale_factor=2, reduced_motion="reduce")
            c.add_init_script("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"); c.add_init_script(CAP)
            q = c.new_page(); q.goto(f"http://127.0.0.1:{r.port}/"); ready(q)
            q.set_input_files("#pick", [str(ROOT / "mzlab" / "web" / "esempi" / "FullScan_t0.mzML")]); attendi(q, "document.querySelectorAll('#flist input[data-k=use]').length>=1")
            q.click("#opbtn"); ready(q); q.click("#np-map"); ready(q)
            q.locator(".pnl.map [data-o=view][data-v='3d']").click(); attendi(q, "document.querySelector('dialog.m3d[open]') && window.__m3d && window.__m3d.V.g", 60000)
            with q.expect_download(timeout=60000) as d: q.evaluate("document.querySelector('dialog.m3d [data-m=png]').click()")
            w, h = dims(d.value.path()); troppo = q.evaluate("window.__troppo"); c.close()
            assert not troppo and w * h <= MAX and w > 2000, (w, h, troppo)
        step("3D map on a 2x screen: the PNG stays under the cap", map3d)
finally:
    r.close()
r.report()
for s in steps: print("  ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
