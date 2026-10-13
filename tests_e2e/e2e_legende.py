# e2e_legende: the legend of a graph never covers the data (a peak at the far right pushes it to another corner; no free corner = headroom)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8876, wd="/tmp/wd_legende")
INK = """(p) => { const el = p.type === 'spec' ? p.leg2 : p.leg; if (!el || !el.offsetWidth) return -1;
  const c = p.cv, b = el.getBoundingClientRect(), cb = c.getBoundingClientRect(), s = c.width / c.offsetWidth;
  const k = document.createElement('canvas').getContext('2d', { willReadFrequently: true }); k.canvas.width = Math.max(1, b.width * s); k.canvas.height = Math.max(1, b.height * s);
  k.drawImage(c, Math.max(0, (b.left - cb.left) * s), Math.max(0, (b.top - cb.top) * s), k.canvas.width, k.canvas.height, 0, 0, k.canvas.width, k.canvas.height);
  const d = k.getImageData(0, 0, k.canvas.width, k.canvas.height).data;
  let n = 0; for (let i = 3; i < d.length; i += 4) if (d[i] > 40) n++; return n / (s * s); }"""
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1})")
        pg.wait_for_timeout(1200)
        def real():
            bad = pg.evaluate("(f) => { const g = eval(f); return stackOrder().filter(q => ['chrom','xic','spec'].includes(q.type)).map(q => [q.type, g(q), q._legPad || 0]); }", INK)
            assert bad, "no panels"
            for t, ink, pad in bad: assert ink <= 3 or pad > 0, (t, ink, pad)
        step("no legend covers the data of the opened panels", real)
        def forced():
            res = pg.evaluate("""(f) => { const q = stackOrder().find(x => x.type === 'spec' && x.leg2 && x.leg2.offsetWidth); if (!q) return null;
              const el = q.leg2, g = q.cv.getContext('2d'), s = q.cv.width / q.cv.offsetWidth;
              const b = el.getBoundingClientRect(), cb = q.cv.getBoundingClientRect();
              g.fillStyle = '#d00'; g.fillRect((b.left - cb.left - 4) * s, 0, (b.width + 8) * s, q.cv.height * 0.55);       // a tall peak right under the legend
              LEGENDE.place(q, el);
              const a = eval(f)(q); return [a, el.style.left]; }""", INK)
            assert res, "no spectrum with a legend"
            assert res[0] <= 3, res
        step("a peak under the legend moves it to a free corner", forced)
        def pad():
            res = pg.evaluate("""() => { const q = stackOrder().find(x => x.type === 'spec' && x.leg2 && x.leg2.offsetWidth); const g = q.cv.getContext('2d');
              g.fillStyle = '#d00'; g.fillRect(0, 0, q.cv.width, q.cv.height); q._legKey = null; return LEGENDE.place(q, q.leg2) && q._legPad > 0; }""")
            assert res, "no headroom requested"
        step("no free corner asks for headroom", pad)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
