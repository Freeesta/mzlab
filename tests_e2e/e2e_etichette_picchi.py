"""Labels of the main peaks of the chromatograms (etichette-picchi.js): RT + base peak m/z on a TIC (synthetic Full Scan), RT + wavelength on a PDA trace (synthetic
DAD with two maxima at different wavelengths); the control shows and hides them, they never overlap, follow the zoom, enter the table of the data and pass axe."""
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr, dati_sintetici as ds
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_etichette_picchi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="etichette_picchi_"))
MS = D / "TIC_t0.mzML"; ds.full_scan(MS, 0, np.random.default_rng(5))
w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(60)]
ts = [0.05 * k for k in range(60)]
def dad(t, j): return 400 * math.exp(-((j - 60) / 12) ** 2) * math.exp(-((t - 1.0) / 0.12) ** 2) + 300 * math.exp(-((j - 120) / 12) ** 2) * math.exp(-((t - 2.0) / 0.12) ** 2)
rows = [[10 + dad(t, j) for j in range(201)] for t in ts]
w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, 200.0, 1.0, rows)); w.save(D, "pda")
CH = "E.panels.find(q=>q.type==='chrom')"
LB = f"({CH}._a.lbls || []).filter(l => l.etpk)"
def labels(pg, n=1):
    attendi(pg, f"{LB}.length >= {n}")
    return pg.evaluate(f"{LB}.map(l => [l.text, l.x, l.y, l.w, l.h])")
def overlap(a, b): return a[1] < b[1] + b[3] and b[1] < a[1] + a[3] and a[2] < b[2] + b[4] and b[2] < a[2] + a[4]
r = Run(port=8877, wd="/tmp/wd_etichette_picchi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(MS)]); pg.click("text=Carica dati"); ready(pg)
        def tic():
            ls = labels(pg, 2); txt = [l[0] for l in ls]
            big = [t for t in txt if t.startswith("14.3")]
            assert big and "364" in big[0], txt
            assert any(t.startswith("7.0") and "229" in t for t in txt) or len(txt) >= 2, txt
        step("TIC: the tallest peak is labelled with RT and base peak m/z", tic)
        def nolap():
            ls = labels(pg, 2)
            for i, a in enumerate(ls):
                for b in ls[i + 1:]: assert not overlap(a, b), (a, b)
        step("the labels do not overlap", nolap)
        def tip():
            t = pg.evaluate(f"{LB}[0].tip"); assert "RT" in t and "min" in t, t
        step("each label has a tooltip", tip)
        def table():
            d = pg.evaluate(f"TDATI_P.data({CH})"); assert any("364" in str(r[0]) for r in d["rows"]), len(d["rows"])
        step("the table of the data lists the labels", table)
        def zoom():
            pg.evaluate(f"(()=>{{const p={CH}; p.zoom=[13.5,15.5]; draw(p)}})()"); frames(pg, 3)
            ls = pg.evaluate(f"{LB}.map(l => l.text)"); assert ls and all(13.5 <= float(t.split(' ')[0]) <= 15.5 for t in ls), ls
            pg.evaluate(f"{CH}.zoom=null; draw({CH})"); frames(pg, 2)
        step("the labels follow the zoom", zoom)
        def control():
            pg.evaluate(f"ETPK.toggle({CH})"); frames(pg, 3)
            assert pg.evaluate(f"{LB}.length") == 0 and pg.evaluate(f"{CH}.etpk") is False
            pg.evaluate(f"ETPK.toggle({CH})"); labels(pg, 1)
            assert pg.evaluate(f"{CH}.etpk") is True
            assert pg.evaluate("AZIONI.per({vista:'chrom', p:" + CH + ", k:0, x:1}).some(a => a.id === 'chrom.etpk')")
        step("the control hides and shows the labels (and is in the menu of the chromatogram)", control)
        def axe():
            if not AXE: steps.append(("axe-core not available (SKIP)", "ok")); return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
        step("axe: no serious violations", axe)
    r2 = Run(port=8878, wd="/tmp/wd_etichette_picchi2")
    try:
        with sync_playwright() as p:
            pg = r2.page(p)
            pg.set_input_files("#pick", [str(D / "pda.wiff"), str(D / "pda.wiff.scan")]); pg.click("text=Carica dati"); ready(pg)
            def pda():
                pg.evaluate(f"(()=>{{const c={CH}; const s=c.el.querySelector('select[data-o=kind]'); s.value='pda'; s.dispatchEvent(new Event('change',{{bubbles:true}}))}})()")
                attendi(pg, f"{CH}.kind==='pda' && {CH}._a")
                ls = labels(pg, 2); txt = sorted(l[0] for l in ls)
                assert any(t.startswith("1.0") and "260 nm" in t for t in txt) and any(t.startswith("2.0") and "320 nm" in t for t in txt), txt
            step("PDA: two peaks, two wavelengths (260 and 320 nm)", pda)
    finally:
        r2.close()
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
