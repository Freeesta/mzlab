"""RT x m/z map of centroided unit-resolution files that declare the same scan window (125-360) but have peaks only in part of it: the m/z axis is the declared
window, the same on every file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT))
from mzlab import demo
M = "E.panels.find(q=>q.type==='map')"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
WIN = ('<scanWindowList count="1"><scanWindow><cvParam cvRef="MS" accession="MS:1000501" name="scan window lower limit" value="125.0"/>'
       '<cvParam cvRef="MS" accession="MS:1000500" name="scan window upper limit" value="360.0"/></scanWindow></scanWindowList>')
def write(path, t0, t1, m0, m1):
    rng = np.random.default_rng(4); specs = []
    for i, rt in enumerate(np.arange(t0, t1 + 1e-9, 0.1)):
        mz = np.sort(rng.uniform(m0, m1, 15)); specs.append(demo._spectrum(i, 1, float(rt), mz, rng.lognormal(8, .5, 15)).replace("</scan></scanList>", WIN + "</scan></scanList>"))
    head = '<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r"><spectrumList count="%d">' % len(specs)
    Path(path).write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")
D = HERE / "shots" / "finestra_tmp"; D.mkdir(parents=True, exist_ok=True)
files = [D / "a-w.mzML", D / "b-w.mzML"]
write(files[0], 0.5, 10.0, 131, 337); write(files[1], 0.5, 10.0, 129, 313)
r = Run(port=8915, wd="/tmp/wd_mappa_finestra")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "!!document.querySelector('#pick')")
        pg.set_input_files("#pick", [str(f) for f in files]); attendi(pg, "!!document.querySelector('#opbtn')")
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, "!!E.panels.find(q=>q.type==='map')")
        shown = lambda: (ready(pg), attendi(pg, f"!!{M}._a && {M}._a.f.k==={M}.k"))
        def axis():
            shown(); a = pg.evaluate(f"({M}._a.y0)"), pg.evaluate(f"({M}._a.y1)")
            assert a[0] <= 125 and a[1] >= 360 and a[0] > 120 and a[1] < 366, a            # the declared window with a small margin, not the peaks (131-337)
            nxt = ".pnl.map [data-a=mapnav][data-d='1']"; lbl = pg.evaluate(f"E.files.find(f=>f.k==={M}.k).label")
            pg.click(nxt); attendi(pg, f"E.files.find(f=>f.k==={M}.k).label!=={lbl!r}"); shown()
            c = pg.evaluate(f"({M}._a.y0)"), pg.evaluate(f"({M}._a.y1)")
            assert c == a, (a, c)
        step("m/z axis is the declared window and does not change with the file", axis)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
import shutil; shutil.rmtree(D, ignore_errors=True)
for s in steps: print(" ", s)
r.report()
