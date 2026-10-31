"""RT x m/z map: changing the file (arrows or menu) keeps both axes identical. The whole view is the union of the data of all the open files of the tab
(not the file shown), and a zoom chosen by the user stays. Three files with different RT and m/z ranges."""
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
def write(path, t0, t1, m0, m1):
    rng = np.random.default_rng(4); specs = []
    for i, rt in enumerate(np.arange(t0, t1 + 1e-9, 0.1)):
        mz = np.sort(rng.uniform(m0, m1, 15)); specs.append(demo._spectrum(i, 1, float(rt), mz, rng.lognormal(8, .5, 15)))
    head = '<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><run id="r"><spectrumList count="%d">' % len(specs)
    Path(path).write_text(head + "".join(specs) + "</spectrumList></run></mzML>", encoding="utf-8")
D = HERE / "shots" / "assi_fissi_tmp"; D.mkdir(parents=True, exist_ok=True)
files = [D / "a-wide.mzML", D / "b-narrow.mzML", D / "c-mid.mzML"]
write(files[0], 0.5, 20.0, 130, 350); write(files[1], 2.0, 10.0, 200, 260); write(files[2], 1.0, 15.0, 130, 295)
r = Run(port=8914, wd="/tmp/wd_mappa_assi_fissi")
view = lambda pg: pg.evaluate(f"(()=>{{const a={M}._a;return {{x:[a.x0,a.x1],y:[a.y0,a.y1]}}}})()")
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        pg = b.new_context(locale="en-US", viewport={"width": 1440, "height": 950}).new_page()
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
        pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.goto(f"http://127.0.0.1:{r.port}/"); attendi(pg, "!!document.querySelector('#pick')")
        pg.set_input_files("#pick", [str(f) for f in files])
        pg.click("#opbtn"); ready(pg); pg.click("#np-map"); ready(pg); attendi(pg, "!!E.panels.find(q=>q.type==='map')")
        pg.evaluate(M + ".el.scrollIntoView({block:'center'})")
        nxt, prv = ".pnl.map [data-a=mapnav][data-d='1']", ".pnl.map [data-a=mapnav][data-d='-1']"
        lab = lambda: pg.evaluate(f"E.files.find(f=>f.k==={M}.k).label")
        shown = lambda: (ready(pg), attendi(pg, f"!!{M}._a && {M}._a.f.k==={M}.k"))        # the map is drawn for the file of the panel
        seen = {}
        def whole():
            shown(); seen[lab()] = view(pg)
            for _ in range(2):
                before = lab(); pg.click(nxt); attendi(pg, f"E.files.find(f=>f.k==={M}.k).label!=={before!r}"); shown(); seen[lab()] = view(pg)
            vs = list(seen.values()); assert len(vs) == 3 and vs[0] == vs[1] == vs[2], seen
            v = vs[0]; assert v["x"][0] < 1 and v["x"][1] > 19.9 and v["y"][0] < 131 and v["y"][1] > 349, v          # the union: the widest file sets the limits
        step("whole view: the same axes on every file, the union of the data", whole)
        def zoomed():
            pg.evaluate(f"{M}.zoom=[3,8];{M}.zoomY=[200,260];draw({M})"); shown(); attendi(pg, f"{M}._a.x0===3")
            pg.click(prv); attendi(pg, f"E.files.find(f=>f.k==={M}.k).label==='b-narrow'"); shown()
            v = view(pg); assert v == {"x": [3, 8], "y": [200, 260]}, v
        step("a chosen zoom stays when the file changes", zoomed)
        def menu():
            pg.locator(".pnl.map select[data-o=k]").select_option(index=0); attendi(pg, f"E.files.find(f=>f.k==={M}.k).label==='a-wide'"); shown()
            assert view(pg) == {"x": [3, 8], "y": [200, 260]}
            pg.evaluate(f"{M}.zoom=null;{M}.zoomY=null;draw({M})"); shown(); attendi(pg, f"{M}._a.x0<1")
            assert view(pg) == seen["a-wide"], (view(pg), seen["a-wide"])
        step("file menu: same behaviour; back to the whole view", menu)
        def scale():
            refs = lambda: pg.evaluate(f"{M}._img.ref")
            own = {}
            for lbl in ("a-wide", "b-narrow", "c-mid"):
                pg.locator(".pnl.map select[data-o=k]").select_option(label=lbl); attendi(pg, f"E.files.find(f=>f.k==={M}.k).label==={lbl!r}"); shown(); own[lbl] = refs()
            assert len(set(own.values())) > 1, own                                        # each file has its own top of the scale
            pg.locator(".pnl.map [data-mz=fix]").evaluate("e => e.click()"); attendi(pg, f"{M}._img.ref!=={own['c-mid']}"); shown()
            fixed = {}
            for lbl in ("a-wide", "b-narrow", "c-mid"):
                pg.locator(".pnl.map select[data-o=k]").select_option(label=lbl); attendi(pg, f"E.files.find(f=>f.k==={M}.k).label==={lbl!r}"); shown(); fixed[lbl] = refs()
            assert len(set(fixed.values())) == 1 and fixed["a-wide"] == max(own.values()), (own, fixed)
        step("fixed colour scale: the same top on every file (the highest of the files)", scale)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
import shutil; shutil.rmtree(D, ignore_errors=True)
for s in steps: print(" ", s)
r.report()
