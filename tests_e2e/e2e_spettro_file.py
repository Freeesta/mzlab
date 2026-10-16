"""File menu of the spectrum panel: it lists only the files ticked in the list; the choice stays after a click in the TIC (the click changes only the RT);
unticking the chosen file moves the panel to the first ticked one."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
SPEC = "E.panels.find(p=>p.type==='spec'&&p.tab==='full')"
CH = "E.panels.find(p=>p.type==='chrom'&&p.tab==='full')"
r = Run(port=8877, wd="/tmp/wd77")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        names = ["B_FullMass-t0", "B_FullMass-t5", "B_FullMass-t10", "B_FullMass-t15", "B_FullMass-t45"]
        stage(pg, [mz(n) for n in names])
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        ks = pg.evaluate("tabFiles('full').map(f=>f.k)")
        assert len(ks) == 5, ks
        pg.evaluate(f"(()=>{{E.browse=false;tabFiles('full').forEach(f=>f.vis=f.k==={ks[0]}||f.k==={ks[1]});renderFileList();redrawAll()}})()")
        attendi(pg, f"!!{SPEC}&&!!{SPEC}._a")
        pg.evaluate(f"{SPEC}.el.scrollIntoView({{block:'center'}})")
        def opts(): return pg.evaluate(f"[...{SPEC}.el.querySelectorAll('select[data-o=k] option')].map(o=>+o.value)")
        def cur(): return pg.evaluate(f"{SPEC}.k")
        def click_tic(rt):
            pg.evaluate(f"{CH}.el.scrollIntoView({{block:'center'}})")
            c = pg.evaluate(f"(()=>{{const p={CH},q=p.cv.getBoundingClientRect();return {{x:q.left+p._a.X({rt}),y:q.top+q.height*0.5}}}})()")
            pg.mouse.move(c["x"], c["y"]); pg.mouse.click(c["x"], c["y"]); pg.wait_for_timeout(700)
        def solo_spuntati(): assert sorted(opts()) == sorted(ks[:2]), (opts(), ks)
        step("menu: only the ticked files", solo_spuntati)
        def persiste():
            pg.select_option(f"select[data-o=k]", str(ks[1])); pg.wait_for_timeout(600)
            assert cur() == ks[1], cur()
            r0 = pg.evaluate(f"{SPEC}.r0")
            click_tic(9.0); r1 = pg.evaluate(f"{SPEC}.r0"); assert r1 != r0 and abs(r1 - 9.0) < 0.5, ("click did not move the RT", r0, r1)
            assert cur() == ks[1], ("after click 1", cur())
            click_tic(12.0); assert abs(pg.evaluate(f"{SPEC}.r0") - 12.0) < 0.5 and cur() == ks[1], ("after click 2", cur())
        step("the choice survives clicks in the TIC", persiste)
        def tolta():
            pg.evaluate(f"(()=>{{E.files[{ks[1]}].vis=false;renderFileList();redrawAll()}})()"); pg.wait_for_timeout(900)
            assert cur() == ks[0], cur()
            assert opts() == [ks[0]], opts()
            assert not pg.evaluate("window.__errs&&window.__errs.length"), "errors"
        step("unticked file: the panel goes to the first ticked one", tolta)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
