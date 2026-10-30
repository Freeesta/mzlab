"""Labels of the main peaks of the chromatograms (etichette-picchi.js): RT + base peak m/z on a TIC (synthetic Full Scan), the time alone on a PDA trace (synthetic
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
            txt = [l[0] for l in labels(pg, 1)]
            big = [t for t in txt if t.startswith("14.3")]
            assert big and float(big[0]) > 0 and "min" not in big[0] and all("·" not in t and "m/z" not in t for t in txt), txt
        step("TIC: the tallest peak is labelled with its time only", tic)
        def zones():
            # the run is 0.5-20 min: no label in the solvent front (first 3 min) or in the last 5 min
            rts = [float(l[0].split(" ")[0]) for l in labels(pg, 1)]
            assert rts and all(3.0 <= t <= 15.2 for t in rts), rts
        step("TIC: no label in the first 3 min or in the last 5 min", zones)
        def cursor_behind():
            # the dashed cursor passes behind a label: its overlay is cut where the label box crosses it
            lb = labels(pg, 1)[0]
            m = pg.evaluate(f"(() => {{ const p = {CH}; p.cur = p._a.x0 + 0 * 1; const l = {LB}[0]; const rt = p._a.x0 + (l.x + l.w / 2 - p._a.X(p._a.x0)) / (p._a.X(p._a.x1) - p._a.X(p._a.x0)) * (p._a.x1 - p._a.x0); p.cur = rt; cursorLine(p); return p.cl.style.maskImage || p.cl.style.webkitMaskImage || ''; }})()")
            assert "transparent" in m, m
        step("the cursor line is cut behind a peak label", cursor_behind)
        def tip():
            t = pg.evaluate(f"{LB}[0].tip"); assert "intensit" in t and "RT" not in t and "Right" not in t and "m/z" not in t, t
        step("each label has a tooltip that adds the intensity (not the time already written)", tip)
        def table():
            d = pg.evaluate(f"TDATI_P.data({CH})"); assert d["rows"] and any("14.3" in str(r[0]) for r in d["rows"]), len(d["rows"])
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
        def at(dx, dy=0):
            # client coordinates of the first label (its centre) moved by dx, dy; the label record and the canvas origin
            return pg.evaluate(f"(() => {{ const p = {CH}, l = {LB}[0], r = p.cv.getBoundingClientRect(); return {{ x: r.left + l.x + l.w / 2 + ({dx}), y: r.top + l.y + l.h / 2 + ({dy}), rt: l.rt, ay: r.top + l.ay, w: l.w }}; }})()")
        def click_label():
            for dx in (0, 5):
                o = at(dx); pg.evaluate(f"{CH}.cur = null"); pg.mouse.click(o["x"], o["y"]); frames(pg, 3)
                cur = pg.evaluate(f"{CH}.cur"); assert cur is not None and abs(cur - o["rt"]) < 1e-3, (dx, cur, o["rt"])
                sp = pg.evaluate("(() => { const s = E.panels.find(q => q.type === 'spec' && q.r0 != null); return s ? (s.r0 + s.r1) / 2 : null })()")
                assert sp is not None and abs(sp - o["rt"]) < 0.05, (dx, sp, o["rt"])
        step("a click on the label (also 5 px to the side) opens the spectrum at the RT of the apex", click_label)
        def spec_label_tip():
            attendi(pg, "(E.panels.find(q => q.type === 'spec')?._a?.lbls || []).some(l => l.m != null)")
            tips = pg.evaluate("E.panels.find(q => q.type === 'spec')._a.lbls.filter(l => l.m != null).map(l => l.tip)")
            assert tips and all("m/z" in t and "intensit" in t and "Right-click" not in t and "Tasto destro" not in t for t in tips), tips[:2]
        step("the label of a spectrum peak shows m/z and intensity, no right-click hint", spec_label_tip)
        def click_peak_strip():
            o = at(0); pg.evaluate(f"{CH}.cur = null"); pg.mouse.click(o["x"] + 6, (o["y"] + o["ay"]) / 2 if o["ay"] > o["y"] + 20 else o["y"]); frames(pg, 3)
        step("a click on the strip between label and apex does not fail", click_peak_strip)
        def click_elsewhere():
            o = at(0); pg.evaluate(f"{CH}.cur = null"); pg.mouse.click(o["x"] + o["w"] / 2 + 60, o["y"] - 4); frames(pg, 3)
            cur = pg.evaluate(f"{CH}.cur"); assert cur is not None and abs(cur - o["rt"]) > 0.05, (cur, o["rt"])
        step("a click away from the label stays at the pixel clicked", click_elsewhere)
        def ctx_label():
            o = at(6); pg.mouse.click(o["x"], o["y"], button="right"); frames(pg, 2)
            t = pg.evaluate("document.querySelector('#ctx').innerText"); assert f"RT {o['rt']:.2f} min" in t, (t[:80], o["rt"])
            pg.keyboard.press("Escape"); frames(pg, 1)
        step("a right click a little beside the label opens the menu of the RT of the peak", ctx_label)
        def hover_label():
            o = at(0); pg.mouse.move(o["x"] + 3, o["y"]); frames(pg, 3)
            t = pg.evaluate(f"{CH}.tip.innerText"); assert "intensit" in t and "Right-click" not in t and "Tasto destro" not in t and "Peak at" not in t and "Picco a" not in t, t
            assert pg.evaluate(f"{CH}.cv.style.cursor") == "pointer"
        step("hovering the label shows the pointer and the intensity, no right-click hint", hover_label)
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
                pick_kind(pg, "pda", CH)
                attendi(pg, f"{CH}.kind==='pda' && {CH}._a")
                attendi(pg, f"{LB}.filter(l => l.etpk && /^\\d+\\.\\d{{2}}$/.test(l.text)).length >= 2")
                ls = labels(pg, 2); txt = sorted(l[0] for l in ls)
                for i, a in enumerate(ls):
                    for b in ls[i + 1:]: assert not overlap(a, b), (a, b)
                assert any(t.startswith("1.0") for t in txt) and any(t.startswith("2.0") for t in txt) and all("nm" not in t for t in txt), txt      # the label is the time alone: the wavelength is in the title and the legend
            step("PDA: two peaks (maxima at 260 and 320 nm), time labels apart", pda)
    finally:
        r2.close()
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
