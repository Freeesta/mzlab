"""Block 1.3 (prompt 3): profile files (line, one peak per nominal mass, line/sticks switch), centroid merge setting, mixed-format note.
Synthetic profile files from tools/dati_sintetici.py (the real profile files are used by tests/test_peaks.py)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_profilo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path("/tmp/prof_e2e"); D.mkdir(exist_ok=True)
ds.full_scan_profile(D / "P_FullMass-t15.mzML", 15, np.random.default_rng(3))
ds.full_scan(D / "C_FullMass-t15.mzML", 15, np.random.default_rng(4))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
r = Run(port=8884, wd="/tmp/wd84")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(D / "P_FullMass-t15.mzML")]); pg.wait_for_timeout(1000)
        assert pg.evaluate("document.querySelector('#modenote').hidden"), "one format only: no note"
        pg.set_input_files("#pick", [str(D / "P_FullMass-t15.mzML"), str(D / "C_FullMass-t15.mzML")]); pg.wait_for_timeout(1200)
        def note():
            assert not pg.evaluate("document.querySelector('#modenote').hidden"), "profile + centroid: the note appears"
            assert "non sono confrontabili" in pg.inner_text("#modenote")
        step("mixed formats: the note on the loading screen", note)
        pg.click("text=Carica dati"); ready(pg)
        pk = pg.evaluate("E.files.findIndex(f=>f.file.startsWith('P_'))"); ck = pg.evaluate("E.files.findIndex(f=>f.file.startsWith('C_'))")
        def show(k, a=14.6, b=14.8):
            pg.evaluate(f"(()=>{{const p={SPEC};p.all=false;p.k={k};p.r0={a};p.r1={b};p.zoom=null;p.si=null;draw(p)}})()"); pg.wait_for_timeout(1500)
        pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})")
        def line():
            show(pk)
            d = pg.evaluate(f"(()=>{{const d={SPEC}._a.data[0].d;return {{mode:d.mode,n:d.pmz.length,np:d.mz.length}}}})()")
            assert d["mode"] == "profile" and d["n"] > 500 and d["np"] < 200, d
            near = pg.evaluate(f"{SPEC}._a.data[0].d.mz.filter(m=>m>=304.8&&m<305.8)"); assert len(near) == 1, near
            lbl = pg.evaluate(f"{SPEC}._a.lbls.filter(l=>l.m>=304.8&&l.m<305.8).length"); assert lbl <= 1
            pg.screenshot(path=SH + "profilo_linea.png", clip=pg.evaluate(f"(()=>{{const q={SPEC}.cv.getBoundingClientRect();return {{x:q.left,y:q.top,width:q.width,height:q.height}}}})()"))
        step("profile file: continuous line, ONE peak for the flat ion", line)
        def switch():
            ink = lambda: pg.evaluate(f"{SPEC}.cv.toDataURL().length")
            a = ink()
            pg.click(f"{SPEC.replace('E.panels.find', 'xx')}" if False else ".pnl.spec [data-a=par]"); pg.wait_for_timeout(300)
            pg.select_option(".sp-pop select[data-s=sticks]", "1"); pg.wait_for_timeout(1200)
            assert pg.evaluate(f"{SPEC}.sticks") is True
            b = ink(); print("ink line/sticks", a, b); assert a != b
            pg.select_option(".sp-pop select[data-s=sticks]", "0"); pg.wait_for_timeout(800)
            pg.keyboard.press("Escape")
        step("switch line / sticks in the parameters", switch)
        def labels_peaks():
            show(pk, 14.2, 14.4)
            m = pg.evaluate(f"{SPEC}._a.lbls.map(l=>l.m).filter(m=>m>360&&m<368)")
            assert sorted({round(x) for x in m}) == sorted({round(x) for x in m}) and len(m) == len(set(round(x) for x in m)), m
        step("labels: one per nominal mass", labels_peaks)
        def centroid():
            pg.evaluate("document.querySelector('#np-set').click()"); pg.wait_for_timeout(300)
            assert pg.is_checked("#uip-merge"), "the merge is on by default"
            show(ck, 14.2, 14.4)
            on = pg.evaluate(f"{SPEC}._a.data[0].d.mz.length")
            pg.uncheck("#uip-merge"); pg.wait_for_timeout(1500)
            off = pg.evaluate(f"{SPEC}._a.data[0].d.mz.length")
            print("centroids merged/not merged", on, off); assert on <= off
            nom = pg.evaluate(f"{SPEC}._a.data[0].d.mz.map(m=>Math.floor(m+0.2))")
            pg.check("#uip-merge"); pg.wait_for_timeout(1500)
            nom2 = pg.evaluate(f"{SPEC}._a.data[0].d.mz.map(m=>Math.floor(m+0.2))"); assert len(nom2) == len(set(nom2)), "merged: one per nominal mass"
        step("centroid file: the merge setting works", centroid)
        pg.screenshot(path=SH + "profilo.png")
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
