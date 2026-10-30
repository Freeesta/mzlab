"""The default m/z axis of an MS2 spectrum fits its own peaks (+5%, at least the precursor + 5), not the range of all the MS2 scans of the file; MS1 keeps the file range."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
r = Run(port=8868, wd="/tmp/wd_ms2_asse")
try:
    with sync_playwright() as p:
        pg = r.page(p); D = synth("ms2_asse")
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg.evaluate("setTab('full')"); ready(pg)
        MS2 = "E.panels.find(p=>p.type==='spec'&&p.level===2&&p._a)"
        pg.evaluate("(async()=>{const D=await DDA.get(E.panels.find(p=>p.level===1&&p.type==='spec').k);await DDA.selectMs2(E.panels.find(p=>p.level===2),D.sid[0]);})(); 0")
        attendi(pg, f"!!({MS2})")
        r1 = pg.evaluate(f"(()=>{{const p={MS2},a=p._a,d=a.data[0].d,mz=d.pmz||d.mz;return {{x0:a.x0,x1:a.x1,lo:mz[0],hi:mz[mz.length-1],prec:a.data[0].prec,fileHi:E.files[p.k].mz2_max}}}})()")
        print(r1)
        assert r1["x1"] <= r1["hi"] * 1.12 + 6 and r1["x1"] >= r1["hi"], r1          # fits the peaks, with a margin
        assert r1["x0"] >= 0 and r1["x0"] <= r1["lo"], r1
        s1 = pg.evaluate("(()=>{const p=E.panels.find(p=>p.type==='spec'&&p.level===1&&p._a);return [p._a.full[0],p._a.full[1],E.files[p.k].mz_min,E.files[p.k].mz_max]})()")
        assert s1[0] <= s1[2] and s1[1] >= s1[3], s1          # MS1: the range of the file
finally:
    r.close(); r.report()
print("\nSTEPS\n  MS2 axis fits the peaks, MS1 keeps the file range: ok")
