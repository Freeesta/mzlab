"""The «# mzLab ...» line of an MS2 spectrum export carries the scan (precursor, activation, CE, RT, scan number, polarity); an MS1 spectrum keeps the short line; copy, CSV and xlsx share it."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load
r = Run(port=8870, wd="/tmp/wd_exp_head")
try:
    with sync_playwright() as p:
        pg = r.page(p); D = synth("exp_head")
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2); pg.evaluate("setTab('full')"); ready(pg)
        pg.evaluate("(async()=>{const s2=E.panels.find(p=>p.level===2),s1=E.panels.find(p=>p.level===1&&p.type==='spec'),D=await DDA.get(s1.k);await DDA.selectMs2(s2,D.sid[0]);})(); 0")
        attendi(pg, "!!E.panels.find(p=>p.level===2&&p._scan&&p._a)")
        h2 = pg.evaluate("(async()=>await PROV.tsv('a\\tb', {p:E.panels.find(p=>p.level===2), table:'plot'}))()")
        line = h2.split("\n")[0]; print(line)
        for k in ("precursor=", "rt=", "scan=", "polarity=", "table=plot"): assert k in line, (k, line)
        assert h2.split("\n")[1] == "a\tb", h2          # the two columns are untouched
        h1 = pg.evaluate("(async()=>await PROV.tsv('a\\tb', {p:E.panels.find(p=>p.level===1&&p.type==='spec'), table:'plot'}))()").split("\n")[0]
        assert "precursor=" not in h1 and "scan=" not in h1, h1
        c = pg.evaluate("JSON.stringify(PROV.ctx({p:E.panels.find(p=>p.level===2), table:'plot'}))")      # copy, CSV and xlsx all start from ctx
        assert '"precursor"' in c
finally:
    r.close(); r.report()
print("\nSTEPS\n  MS2 export header has the scan fields, MS1 has not: ok")
