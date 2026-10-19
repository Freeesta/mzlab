"""RT x nm map of the PDA of a SCIEX .wiff in the browser: the engine reads the optical detector of the pair, the map offers the PDA choice, draws the matrix with
the wavelength axis in nm and the table of the data (RT, nm, mAU) is there. Synthetic pair (tools/wiff_riferimento.py), no laboratory data."""
import sys, os, math, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import wiff_riferimento as wr
AXE = axe_path()
RUN = "axe.run(%s, {runOnly: {type: 'tag', values: ['wcag2a','wcag2aa','wcag21a','wcag21aa','wcag22aa']}}).then(r => r.violations.filter(v => ['serious','critical'].includes(v.impact)).map(v => [v.id, v.impact, v.nodes.length]))"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_pda_wiff.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="mappa_pda_wiff_"))
w = wr.Writer(); blocks = [("scan", [(100 + k, 50 + k)], 1000.0 * k, 50.0 + k) for k in range(40)]
ts = [0.05 * k for k in range(40)]; rows = [[10 + 400 * math.exp(-((j - 60) / 12) ** 2) * math.exp(-((t - 1.0) / 0.2) ** 2) for j in range(201)] for t in ts]
w.sample(1, blocks, {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}, dad=(ts, 200.0, 1.0, rows)); w.save(D, "uv")
r = Run(port=8867, wd="/tmp/wd_mappa_pda_wiff")
M = "E.panels.find(q=>q.type==='map')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "uv.wiff"), str(D / "uv.wiff.scan")])
        pg.click("text=Carica dati"); ready(pg)
        def offered():
            assert pg.evaluate("E.files[0].pda") is True
            pg.click("#np-map"); ready(pg); attendi(pg, "E.files[0]._pdaMap === true")
            attendi(pg, f"!!{M}.el.querySelector('select[data-o=mapKind]')")
        step("the map offers MS / PDA for a .wiff with the optical detector", offered)
        def drawn():
            pg.select_option(f"#{{}} select[data-o=mapKind]".replace("#{}", "body"), "pda"); ready(pg)
            attendi(pg, f"{M}._a && {M}._a.map && {M}._a.mat && {M}._a.mat.nmz === 201")
            a = pg.evaluate(f"({{nrt: {M}._a.mat.nrt, mz0: {M}._a.mat.mz0, dmz: {M}._a.mat.dmz, mx: Math.max(...{M}._a.mat.v)}})")
            assert a["nrt"] == 40 and a["mz0"] == 200 and a["dmz"] == 1 and a["mx"] > 300, a
        step("choosing PDA draws the matrix: 40 times x 201 nm, with the peak in it", drawn)
        def table():
            d = pg.evaluate(f"TDATI_P.data({M})"); t = pg.evaluate(f"TDATI_P.text({M})")
            assert d["head"][1:] == ["nm", "Assorbanza (mAU)"] and len(d["rows"]) > 0, d["head"]
            assert any(abs(float(r[1]) - 260) <= 2 for r in d["rows"][:5]), d["rows"][:5]       # the strongest cells sit at the peak (200 + 60 nm)
            assert "nm" in t and "m/z" not in t, t
        step("the table of the data has RT, nm and mAU; the description speaks of wavelengths", table)
        def axe():
            if not AXE: steps.append(("axe-core not available (SKIP)", "ok")); return
            pg.add_script_tag(path=AXE)
            v = pg.evaluate(RUN % "'#dpanels'"); assert not v, v
        step("axe-core: no serious or critical violation on the PDA map panel", axe)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
