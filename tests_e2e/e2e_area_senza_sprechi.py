"""C29: from «Carica dati» to the work area, nothing is wasted. Checked by STATE (no times):
(1) no graph is set up (canvas sized, context made) while #v-data is still hidden;
(2) the first visible TIC is drawn from the provisional TIC of the indexing, with the real api/chrom still on its way (held back here until the test lets it go);
(3) api/chrom is asked for before api/metodo (the series list of an instrument log waits for it), and once per chromatogram, not once per redraw."""
import os, sys, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
import dati_sintetici as ds

HOOK = """
() => {
  window.__urls = []; window.__held = []; window.__hold = true; window.__setupHidden = 0;
  const f = window.fetch;
  window.fetch = function (u, ...a) {
    const s = String((u && u.url) || u);
    if (/api\\/(chrom|metodo)/.test(s)) window.__urls.push(s);
    if (window.__hold && /api\\/chrom/.test(s)) return new Promise((res, rej) => window.__held.push(() => f.call(window, u, ...a).then(res, rej)));
    return f.call(window, u, ...a);
  };
  const g = window.setup;
  window.setup = function (c) { if (document.querySelector('#v-data').hidden) window.__setupHidden++; return g.apply(this, arguments); };
}
"""
DRAWN = """
() => { const p = E.panels.find(q => q.type === 'chrom'); if (!p || !p.cv.width) return false;
  const d = p.cv.getContext('2d').getImageData(0, 0, p.cv.width, p.cv.height).data; for (let i = 3; i < d.length; i += 4) if (d[i]) return true; return false; }
"""

r = Run(port=8899, wd="/tmp/wd99")
tmp = Path(tempfile.mkdtemp(prefix="c29s_"))
try:
    f = tmp / "B_FullMass-t0.mzML"; ds.full_scan(f, 0, np.random.default_rng(1))
    with sync_playwright() as p:
        pg = r.page(p)
        attendi(pg, "window.qqReady === true"); ready(pg)
        stage(pg, [f]); ready(pg)
        assert pg.evaluate("!!MZLAB_LR.earlyTic(%r)" % f.name), "no provisional TIC from the indexing: the test would prove nothing"
        pg.evaluate(HOOK)
        pg.click("#opbtn")
        attendi(pg, "window.__held.length > 0")
        attendi(pg, "(" + DRAWN.strip() + ")()")
        assert pg.evaluate("window.__held.length") == 1, "one api/chrom request for the one chromatogram, not one per redraw"
        assert pg.evaluate("window.__setupHidden") == 0, "a graph was set up while the work area was hidden"
        pg.evaluate("window.__hold = false; window.__held.splice(0).forEach(g => g())")
        ready(pg)
        urls = pg.evaluate("window.__urls")
        ic = next(i for i, u in enumerate(urls) if "api/chrom" in u)
        im = next((i for i, u in enumerate(urls) if "api/metodo" in u), len(urls))
        assert ic < im, f"api/metodo asked before api/chrom: {urls}"
        n = sum(1 for u in urls if "kind=tic" in u and "k=0" in u)
        assert n == 1, f"api/chrom for the TIC asked {n} times"
        print("OK  no setup while hidden; provisional TIC drawn before api/chrom; chrom before metodo; one chrom request")
        assert not pg.evaluate("window.MZLAB_LR_MISS && window.MZLAB_LR_MISS.length")
finally:
    r.close()
    import shutil; shutil.rmtree(tmp, ignore_errors=True)
