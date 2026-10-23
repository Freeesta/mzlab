"""UV spectrum of the DAD: a double click on the PDA map or on the PDA trace of a SCIEX .wiff opens the UV spectrum (nm, mAU) of that time under the graph, with its
table of the data; a file with only the PDA trace (no optical matrix) answers with a clear message and opens nothing. Synthetic pair (tools/wiff_riferimento.py)."""
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
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_uv_spettro.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="uv_spettro_"))
w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(40)]
ts = [0.05 * k for k in range(40)]; rows = [[10 + 400 * math.exp(-((j - 60) / 12) ** 2) * math.exp(-((t - 1.0) / 0.2) ** 2) for j in range(201)] for t in ts]
w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, 200.0, 1.0, rows)); w.save(D, "uv")
T0 = D / "SOLOTRACCIA_t0.mzML"; ds.full_scan(T0, 0, np.random.default_rng(5))
UV = "E.panels.find(q=>q.type==='spec'&&q.uv)"
def dbl(pg, sel_js, rt):
    """double click on the panel `sel_js` (a JS expression) at the time `rt` of its x axis, in the middle of the plot"""
    box = pg.evaluate(f"(()=>{{const p={sel_js},b=p.cv.getBoundingClientRect(),a=p._a;return {{x:b.x+a.X({rt}),y:b.y+M.t+(a.H-M.t-M.b)/2}}}})()")
    pg.mouse.dblclick(box["x"], box["y"])
r = Run(port=8868, wd="/tmp/wd_uv_spettro")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "uv.wiff"), str(D / "uv.wiff.scan")])
        pg.click("text=Carica dati"); ready(pg)
        def map_dbl():
            pg.click("#np-map"); ready(pg); attendi(pg, "E.files[0]._pdaMap === true")
            pg.select_option("body select[data-o=mapKind]", "pda"); ready(pg)
            attendi(pg, "E.panels.find(q=>q.type==='map')._a && E.panels.find(q=>q.type==='map')._a.pda")
            dbl(pg, "E.panels.find(q=>q.type==='map')", 1.0)
            attendi(pg, f"!!({UV} && {UV}._a && {UV}._a.uvd)")
            a = pg.evaluate(f"({{n: {UV}._a.uvd.nm.length, nm0: {UV}._a.uvd.nm[0], rt: {UV}._a.uvd.rt, mx: {UV}._a.uvd.nm[{UV}._a.uvd.y.indexOf(Math.max(...{UV}._a.uvd.y))]}})")
            assert a["n"] == 201 and a["nm0"] == 200 and abs(a["rt"] - 1.0) < 0.06 and abs(a["mx"] - 260) <= 1, a
        step("double click on the PDA map: the UV spectrum of that time, peak at 260 nm", map_dbl)
        def table():
            d = pg.evaluate(f"TDATI_P.data({UV})"); t = pg.evaluate(f"TDATI_P.text({UV})")
            assert d["head"] == ["nm", "Assorbanza (mAU)"] and len(d["rows"]) > 50, d["head"]
            assert "UV" in t and "m/z" not in t, t
        step("the table of the data has nm and mAU; the description speaks of the UV spectrum", table)
        def sheet():
            s = pg.evaluate(f"plotSheets({UV})")
            assert s and s[0]["head"][0] == "nm" and s[0]["head"][1] == "Assorbanza (mAU)" and len(s[0]["rows"]) == 201, s and s[0]["head"]
        step("the export has the nm and mAU columns", sheet)
        def zoom_keys():
            pg.evaluate(f"{UV}.cv.focus()"); pg.keyboard.press("+"); frames(pg, 3)
            z = pg.evaluate(f"{UV}.zoom"); assert z and z[1] - z[0] < 200, z
        step("the keyboard zoom works on the UV spectrum", zoom_keys)
        def chrom_dbl():
            n0 = pg.evaluate("E.panels.filter(q=>q.uv).length")
            pg.evaluate("(()=>{const c=E.panels.find(q=>q.type==='chrom'); const s=c.el.querySelector('select[data-o=kind]'); s.value='pda'; s.dispatchEvent(new Event('change',{bubbles:true}))})()")
            ready(pg); attendi(pg, "E.panels.find(q=>q.type==='chrom')._a && E.panels.find(q=>q.type==='chrom').kind==='pda'")
            dbl(pg, "E.panels.find(q=>q.type==='chrom')", 0.5)
            attendi(pg, f"E.panels.filter(q=>q.uv&&q._a).length === {n0 + 1}")
            rt = pg.evaluate("(()=>{const c=E.panels.find(q=>q.type==='chrom'); return E.panels.find(q=>q.uv&&q.src===c.id)._a.uvd.rt})()"); assert abs(rt - 0.5) < 0.06, rt
        step("double click on the PDA trace: the UV spectrum of that time", chrom_dbl)
        def axe():
            if not AXE: steps.append(("axe-core not available (SKIP)", "ok")); return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
        step("axe-core: no serious or critical violation with the UV spectrum open", axe)
        pg.close()
        pg = r.page(p)
        stage(pg, [T0]); pg.click("text=Carica dati"); ready(pg)
        def no_matrix():
            assert pg.evaluate("E.files[0].pda") is True
            pg.select_option("body select[data-o=kind]", "pda"); ready(pg)
            attendi(pg, "E.panels.find(q=>q.type==='chrom')._a && E.panels.find(q=>q.type==='chrom').kind==='pda'")
            dbl(pg, "E.panels.find(q=>q.type==='chrom')", 10)
            attendi(pg, "/DAD/.test(document.body.innerText)")
            assert pg.evaluate("E.panels.filter(q=>q.uv).length") == 0
        step("a file with only the PDA trace: a clear message and no panel", no_matrix)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
