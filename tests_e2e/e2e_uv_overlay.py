"""UV spectrum of the DAD from mzML files with the matrix as `MS:1000804` spectra (the layout of a variant .wiff through the sciex module): a double click on the
PDA trace of several overlaid samples opens one UV spectrum panel with one trace per sample (x = the wavelength range); a second double click asks the engine
nothing (the matrix is held); from a single-wavelength trace the spectrum marks that wavelength; a double click on the UV spectrum gives the trace at that
wavelength. Synthetic files (tools/dati_sintetici_uv.py)."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from pathlib import Path
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici_uv as du
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_uv_overlay.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path(tempfile.mkdtemp(prefix="uv_overlay_"))
FILES = [D / f"DAD_s{i}.mzML" for i in (1, 2)]
for i, f in enumerate(FILES): du.write(f, n_uv=240, n_wl=121, n_scans=120, n_peaks=40, seed=i + 1, minutes=20.0)     # 190-310 nm
C = "E.panels.find(q=>q.type==='chrom'&&q.kind==='pda'&&q.mz0==null)"
UVS = "E.panels.filter(q=>q.uv)"
def dbl(pg, sel_js, x):
    """double click on the panel `sel_js` at `x` of its x axis, in the middle of the plot"""
    box = pg.evaluate(f"(()=>{{const p={sel_js};p.cv.scrollIntoView({{block:'center'}});const b=p.cv.getBoundingClientRect(),a=p._a;return {{x:b.x+a.X({x}),y:b.y+M.t+(a.H-M.t-M.b)/2}}}})()")
    pg.mouse.dblclick(box["x"], box["y"])
r = Run(port=8869, wd="/tmp/wd_uv_overlay")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, FILES); open_first(pg)
        pg.evaluate("E.files.forEach(f=>f.vis=true); syncBrowse(); redrawAll()"); ready(pg)
        def overlay():
            assert pg.evaluate("E.files.filter(f=>f.pda).length") == 2
            pick_kind(pg, "pda"); ready(pg); attendi(pg, f"!!({C} && {C}._a)")
            dbl(pg, C, 6.4)
            attendi(pg, f"{UVS}.length === 1 && {UVS}[0]._a && {UVS}[0]._a.uvd")
            u = f"{UVS}[0]"
            a = pg.evaluate(f"({{full: {u}._a.full, n: {u}._a.uvd.ser.length, ks: {u}._a.uvd.ser.map(s=>s.k), rt: {u}._a.uvd.rt}})")
            assert a["full"] == [190, 310], a
            assert a["n"] == 2 and sorted(a["ks"]) == [0, 1], a
            assert abs(a["rt"] - 6.4) < 0.1, a
            leg = pg.evaluate(f"{u}.leg.textContent"); assert "DAD_s1" in leg and "DAD_s2" in leg, leg
            assert pg.evaluate(f"{u}.el.querySelector('.ttl').textContent") == "Spettro UV"
            assert pg.evaluate(f"{u}.el.querySelector('.rtl').textContent") == "%.2f min" % a["rt"]
        step("double click on the PDA trace of two samples: one UV spectrum panel, one trace per sample, x = 190-310 nm, title «Spettro UV» + RT", overlay)
        def table():
            d = pg.evaluate(f"TDATI_P.data({UVS}[0])")
            assert d["head"][0] == "nm" and len(d["head"]) == 3 and all("Assorbanza (mAU)" in h for h in d["head"][1:]), d["head"]
            s = pg.evaluate(f"plotSheets({UVS}[0])"); assert len(s[0]["head"]) == 4, s[0]["head"]
        step("the table of the data and the export have one absorbance column per sample", table)
        def warm():
            pg.evaluate("(()=>{window.__n=[]; if(!window.__f){window.__f=window.fetch; window.fetch=function(u,...a){ if(/api\\/(uvspec|map|chrom)/.test(String(u))) window.__n.push(String(u)); return window.__f.call(this,u,...a)}}})()")
            dbl(pg, C, 8.5)
            attendi(pg, f"{UVS}.length === 2 && {UVS}[1]._a && Math.abs({UVS}[1]._a.uvd.rt - 8.5) < 0.1")
            assert pg.evaluate("window.__n.filter(u=>/uvspec|map/.test(u))") == [], pg.evaluate("window.__n")
        step("a second double click asks the engine nothing: the matrix is held", warm)
        def from_wl():
            pg.evaluate(f"wlOpen({C}, 254, 254)")
            W = "E.panels.find(q=>q.type==='chrom'&&q.kind==='pda'&&q.mz0===254)"
            attendi(pg, f"!!({W} && {W}._a)")
            n0 = pg.evaluate(f"{UVS}.length")
            dbl(pg, W, 3.1)
            u = f"{UVS}.reduce((a,q)=>q.id>a.id?q:a)"                                     # the newest: a panel opens next to its source, not at the end
            attendi(pg, f"{UVS}.length === {n0 + 1} && {u}._a")
            got = pg.evaluate(f"[{u}.wl, {u}._a.full]")
            assert got == [[254, 254], [190, 310]], got
        step("from the 254 nm trace: the full UV spectrum at that time, with 254 nm marked", from_wl)
        def back_to_wl():
            n0 = pg.evaluate("E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length")
            dbl(pg, f"{UVS}[0]", 280)
            attendi(pg, f"E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length === {n0 + 1}")
            w = pg.evaluate("(()=>{const c=E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop(); return [c.mz0, c.mz1]})()")
            assert w[0] == w[1] and abs(w[0] - 280) < 2, w
            attendi(pg, "E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop()._a")
        step("double click on the UV spectrum: the trace of the DAD at that wavelength", back_to_wl)
        def toolbar():
            n0 = pg.evaluate("E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length")
            assert pg.evaluate(f"{C}.el.querySelector('[data-a=xic]').textContent") == "λ"
            pg.evaluate(f"{C}.el.querySelector('[data-a=xic]').click()")
            attendi(pg, f"E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').length === {n0 + 1} && E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop()._a")
            assert pg.evaluate("E.panels.filter(q=>q.type==='chrom'&&q.kind==='pda').pop()._a.sr.length") == 2
        step("the λ button of the PDA trace opens the trace by wavelength of both samples", toolbar)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report()
