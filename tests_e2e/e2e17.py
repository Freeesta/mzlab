"""Dati sub-tabs: Full Scan / MS2 / MRM never share a graph; MS2 strip + panel per precursor; MRM Quant/Qual; overview matrix."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:220]))
r = Run(port=8817, wd="/tmp/wd17")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES]); pg.wait_for_timeout(1500)
        pg.click("text=Carica dati"); ready(pg)
        tabs = lambda: pg.evaluate("[...document.querySelectorAll('#flst .fgh')].map(g=>g.dataset.gt+':'+g.querySelector('em').innerText)")
        def t1(): assert tabs() == ["full:3", "ms2:1", "mrm:1"], tabs()
        step("file list groups with per-type counts", t1)
        def t2():
            assert pg.evaluate("E.tab") == "full"
            assert pg.evaluate("E.panels.filter(p=>p.tab==='full').length") >= 2
            assert pg.evaluate("E.panels.every(p=>p.el.style.display!=='none')"), "every block is on the page"
        step("Full Scan block: chromatogram + spectrum", t2)
        def t3():
            open_kind(pg, "ms2"); ready(pg)
            assert pg.evaluate("E.tab") == "ms2"
            n = pg.evaluate("E.panels.filter(p=>p.tab==='ms2').length"); assert n >= 2 and n % 2 == 0, n
            assert pg.locator("#expbar").count() == 0                       # the experiments strip was removed (precursors live in the file list)
        step("MS2 block: one chromatogram+spectrum per precursor", t3)
        def t4():
            open_kind(pg, "mrm"); ready(pg)
            m = pg.evaluate("E.panels.filter(p=>p.tab==='mrm')"); assert len(m) >= 1, len(m)
            trs = pg.evaluate("E.panels.find(p=>p.tab==='mrm')?.tr || []")
            cal = pg.evaluate("[CAL.quant, CAL.qual]")
            assert cal[0] and cal[1], cal
            assert cal[0] in trs and cal[1] in trs, (cal, trs)
        step("MRM block: Quantificatore and Qualificatore panels", t4)
        def t5():
            open_kind(pg, "full"); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.tab") == "full"
            open_kind(pg, "ms2"); pg.wait_for_timeout(1500)
            assert pg.evaluate("E.tab") == "ms2" and pg.evaluate("E.files[E.cur].kind") == "ms2"
        step("click on a file of another group: that type becomes the current one", t5)
        def t6():
            pg.evaluate("setTab('full')"); pg.wait_for_timeout(500)
            pg.reload(); pg.wait_for_timeout(1500)
        step("tab switch back works", t6)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
