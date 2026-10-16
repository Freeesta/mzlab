# e2e_mrm_layout: a file with only MRM opens only the MRM block (no empty graphs); the transitions menu has «all transitions» plus the single ones, nothing else
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
VIS = "[...document.querySelectorAll('.pnl')].filter(e => e.offsetParent).map(e => [...e.classList].find(c => ['chrom','xic','spec','mrm','map'].includes(c)))"
r = Run(port=8879, wd="/tmp/wd_mrm_layout")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz("B_MRM-STD_0_6ppm"), mz("B_MRM-t0")])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg); pg.wait_for_timeout(1500)
        def only_mrm():
            v = pg.evaluate(VIS); assert v == ["mrm"], v
            assert pg.evaluate("E.panels.every(q => q.tab === 'mrm')"), "panels of another type"
        step("MRM-only files: only the MRM panel is open", only_mrm)
        def menu():
            o = pg.evaluate("(() => { const s = document.querySelector('.pnl.mrm select[data-o=tr]'); return [...s.options].map(x => [x.value, x.textContent]); })()")
            assert o[0][0] == "" and len(o) > 1, o
            assert not any(v == "__quant_qual" for v, _ in o), o
            assert len({t for _, t in o}) == len(o), o
        step("transitions menu: all + single, no duplicate entry", menu)
        def mixed():
            pg2 = r.page(p)
            pg2.set_input_files("#pick", [mz("B_FullMass-t0"), mz("B_MRM-t0")])
            pg2.wait_for_timeout(1000); pg2.click("text=Carica dati"); ready(pg2); pg2.wait_for_timeout(1500)
            v = pg2.evaluate(VIS); assert "mrm" not in v and "chrom" in v and "spec" in v, v         # first load: only the first file of the list is open
            pg2.evaluate("setVisOf(E.files.filter(f => f.kind === 'mrm'), true)"); ready(pg2)         # ticking the MRM file adds its block
            v = pg2.evaluate(VIS); assert "mrm" in v and "chrom" in v and "spec" in v, v
        step("full scan + MRM: the blocks with data stay", mixed)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
