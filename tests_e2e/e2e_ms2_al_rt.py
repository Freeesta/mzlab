"""«MS2 at this RT…» in the TIC menu of a DDA file: it is there, and it puts the MS2 scan nearest in time in the MS2 panel; a file without MS2 does not show it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
r = Run(port=8872, wd="/tmp/wd_ms2_al_rt")
try:
    with sync_playwright() as p:
        pg = r.page(p); D = synth("ms2_al_rt")
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg.evaluate("setTab('full')"); ready(pg)
        attendi(pg, "!!E.panels.find(p=>p.type==='chrom'&&p._a)")
        pg.evaluate("window.__c = (()=>{const p=E.panels.find(q=>q.type==='chrom'),f=E.files[p.k];return {vista:'chrom',p,x:0.5,k:E.files.find(f=>f.kind==='full').k,px:10,py:10,e:{}}})(); 0")
        ROW = "AZIONI.per(__c).find(i=>i.id==='chrom.ms2AtRt')"
        assert pg.evaluate(f"!!({ROW})"), "the TIC menu has the item"
        assert pg.evaluate(f"({ROW}).tip.length>20"), "with its hint on the triangles"
        pg.evaluate(f"({ROW}).fn(); 0")
        attendi(pg, "!!E.panels.find(p=>p.level===2&&p.sid!=null&&p._scan)")
        rt = pg.evaluate("E.panels.find(p=>p.level===2)._scan.rt"); assert abs(rt - 0.5) < 0.5, rt
        pg.evaluate("E.files[__c.k].dda = false; 0")      # a file without MS2: no item
        assert not pg.evaluate(f"!!({ROW})"), "hidden without MS2"
finally:
    r.close(); r.report()
print("\nSTEPS\n  MS2 at this RT: ok")
