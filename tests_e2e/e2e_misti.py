"""Block I: mixed files (several experiments in ONE file) are split in parts: Full Scan + MS2 (IDA), MRM + EPI, alternating polarity."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_misti.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
D = pathlib.Path(__file__).resolve().parent / "shots" / "misti_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.mixed_ida(D / "C_IDA-t0.mzML", np.random.default_rng(1)); ds.mixed_mrm_epi(D / "C_MRM-EPI-t0.mzML", np.random.default_rng(2)); ds.mixed_polarity(D / "C_POLALT-t0.mzML", np.random.default_rng(3))
r = Run(port=8882, wd="/tmp/wd82", motore="python")
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "C_IDA-t0.mzML"), str(D / "C_MRM-EPI-t0.mzML"), str(D / "C_POLALT-t0.mzML")]); pg.wait_for_timeout(1500)
        def table():
            t = pg.inner_text("#flist"); assert "misto" not in t and "precursor" not in t and "Full Scan + DDA" in t and "MS2 + MRM" in t, t
            assert "Full Scan + MS2" not in t and "pos" not in t and "neg" not in t, t     # IDA = Full Scan + DDA; alternating polarity = a plain Full Scan
            assert pg.evaluate("[...document.querySelectorAll('#flist td')].some(td=>td.textContent.trim()==='Full Scan')"), t
            assert "il nome fa pensare" not in t
        step("loading table: mixed files say what they hold", table)
        pg.click("text=Carica dati"); ready(pg)
        def parts():
            fl = pg.evaluate("E.files.map(f=>[f.file,f.kind,f.label,f.polarity])"); print(fl)
            names = [x[0] for x in fl]
            assert sorted(names) == sorted(["C_IDA-t0.mzML#MS1", "C_IDA-t0.mzML#MS2", "C_MRM-EPI-t0.mzML#MS2", "C_MRM-EPI-t0.mzML#MRM", "C_POLALT-t0.mzML#MS1 pos", "C_POLALT-t0.mzML#MS1 neg"]), names
            kd = dict((x[0], x[1]) for x in fl); assert kd["C_IDA-t0.mzML#MS1"] == "full" and kd["C_IDA-t0.mzML#MS2"] == "ms2" and kd["C_MRM-EPI-t0.mzML#MRM"] == "mrm" and kd["C_MRM-EPI-t0.mzML#MS2"] == "ms2"
            pl = dict((x[0], x[3]) for x in fl); assert pl["C_POLALT-t0.mzML#MS1 pos"] == "positive" and pl["C_POLALT-t0.mzML#MS1 neg"] == "negative", fl
        step("each experiment is its own entry in its own tab, polarities apart", parts)
        def full_tab():
            pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(2500)
            sr = pg.evaluate(f"{CH}._a.sr.map(s=>[s.name,s.x.length])"); print(sr)
            assert len(sr) == 3 and all(n > 100 for _, n in sr), sr
        step("Full Scan tab: the survey of the IDA file and the two polarity parts (3 traces)", full_tab)
        def tri():
            pg.evaluate(f"(()=>{{const p={CH};p.ms2tri=true;ctl(p);draw(p)}})()"); pg.wait_for_timeout(1200)
            n = pg.evaluate(f"{CH}._a.tri.length"); assert n > 20, n
            t = pg.evaluate(f"(()=>{{const p={CH},q=p._a.tri[5];p.el.scrollIntoView({{block:'center'}});const c=p.cv.getBoundingClientRect();return {{x:c.left+q.px,y:c.top+M.t+4,rt:q.rt,prec:q.prec}}}})()")
            pg.mouse.click(t["x"], t["y"]); pg.wait_for_timeout(4500)
            assert pg.evaluate("E.tab") == "ms2", "the triangle opens the MS2 tab"
            s = pg.evaluate("(()=>{const sp=E.panels.find(p=>p.tab==='ms2'&&p.type==='spec');return {rt:(sp.r0+sp.r1)/2,prec:sp.prec,ok:!!sp._a}})()"); print(t, s)
            assert abs(s["rt"] - t["rt"]) < 0.03 and s["ok"], (t, s)
        step("triangles on the survey chromatogram; a click opens that MS2 scan", tri)
        def ms2tab():
            assert pg.evaluate("tabFiles('ms2').length") == 2
            ex = pg.evaluate("ms2Exps().map(e=>[e.prec,e.n])"); print(ex)
            assert len(ex) >= 8 and all(ex[i][1] >= ex[i + 1][1] for i in range(len(ex) - 1)), ex
        step("MS2 tab: precursors grouped and sorted by number of scans", ms2tab)
        def mrmtab():
            pg.evaluate("setTab('mrm',true)"); pg.wait_for_timeout(3000)
            assert pg.evaluate("tabFiles('mrm').length") == 1 and pg.evaluate("E.panels.filter(p=>p.tab==='mrm').length") >= 1
        step("MRM tab gets the MRM part of the MRM + EPI file", mrmtab)
        def reload():
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(1500); pg.reload(); pg.wait_for_timeout(6000)
            assert pg.evaluate("E.files.length") == 6 and pg.evaluate("E.files.map(f=>f.file).includes('C_POLALT-t0.mzML#MS1 neg')")
        step("the session comes back with the same parts", reload)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
