"""Mass-spectrum peak labels never sit under a bar: for every label, no bar inside its horizontal span rises above the text (also when a taller bar is cut by the y zoom)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_etichetta_picco.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("etichetta_picco")
r = Run(port=8893, wd="/tmp/wd_etichetta_picco")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        K = pg.evaluate("E.files.find(f=>f.file==='HR_DDA-Exploris-t30.mzML#MS1').k")
        sp = pg.evaluate(f"(()=>{{const c=E.panels.find(p=>p.type==='chrom'&&p.tab==='full');return newSpec(c,13.9,14.7,{K}).id}})()")
        pg.wait_for_function(f"(()=>{{const p=E.panels.find(q=>q.id=={sp});return p&&p._a&&p._a.data&&p._a.data.length}})()", timeout=20000); pg.wait_for_timeout(400)
        # worst offender over the label boxes: how far (px) a bar inside the span rises above the bottom of the text
        PROBE = f"""(() => {{ const p = E.panels.find(q => q.id == {sp}), a = p._a, d = a.data[0].d; let worst = -1e9, n = 0;
            for (const l of a.lbls) {{ n++; for (let j = 0; j < d.mz.length; j++) {{ const x = a.X(d.mz[j]); if (x < l.x || x > l.x + l.w || d.y[j] <= 0) continue;
              worst = Math.max(worst, (l.y + l.h) - a.Y(Math.min(d.y[j], a.ymax))); }} }}
            return {{ n, worst }}; }})()"""
        def probe(zoom, zy):
            pg.evaluate(f"(()=>{{const p=E.panels.find(q=>q.id=={sp});p.zoom={zoom};p.zoomY={zy};p.thr=0.5;p.nlab=40;draw(p)}})()"); pg.wait_for_timeout(500)
            return pg.evaluate(PROBE)
        def clear():
            for zoom, zy in [("null", "null"), ("[300,520]", "null"), ("[300,520]", "[0,0.3*E.panels.find(q=>q.id==%d)._a.yfull[1]]" % sp), ("[380,440]", "[0,0.1*E.panels.find(q=>q.id==%d)._a.yfull[1]]" % sp)]:
                v = probe(zoom, zy); print(zoom, zy[:12], v)
                assert v["n"] > 0 and v["worst"] <= 0, (zoom, v)
        step("no bar rises into a label (automatic axis and a cut y zoom)", clear)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
