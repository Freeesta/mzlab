# e2e_pannelli_ciclo: opening and closing 50 panels of three kinds leaves no listener behind on document/window (closePanel aborts p._ac)
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8841, wd="/tmp/wd_pciclo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        cdp = pg.context.new_cdp_session(pg)
        def count():
            n = 0
            for expr in ("window", "document"):
                o = cdp.send("Runtime.evaluate", {"expression": expr})["result"]["objectId"]
                n += len(cdp.send("DOMDebugger.getEventListeners", {"objectId": o})["listeners"])
            return n
        KINDS = ["addPanel('spec',{tab:E.tab,k:E.cur,level:1})", "addPanel('chrom',{tab:E.tab,k:E.cur,kind:'tic'})", "addPanel('xic',{traces:[{id:E.seq++,mz:364.4,w:0.5,label:'m/z 364'}]})"]      # the map has its own step below (it needs the engine)
        def cycle(i=0):
            pg.evaluate(KINDS[i % len(KINDS)] + "; closePanel(E.panels[E.panels.length-1])")
            pg.wait_for_timeout(50)
        def leak():
            for i in range(3): cycle(i)                  # one of each kind first: the lazily created listeners exist before the baseline
            base = count()
            for i in range(50): cycle(i)
            assert count() == base, (base, count())
            assert pg.evaluate("STATO.verifica()") == []
        def keyups():
            o = cdp.send("Runtime.evaluate", {"expression": "document"})["result"]["objectId"]
            return sum(1 for l in cdp.send("DOMDebugger.getEventListeners", {"objectId": o})["listeners"] if l["type"] == "keyup")
        def map_cycle():
            pg.evaluate("addPanel('map',{tab:'full'})"); attendi(pg, "E.panels.some(q=>q.type==='map'&&q._a)")
            pg.evaluate("closePanel(E.panels.find(q=>q.type==='map'))")
        def map_leak():
            map_cycle(); base = keyups()
            for _ in range(10): map_cycle()
            assert keyups() == base, (base, keyups())
        step("50 open/close cycles (spec, chrom, xic): listeners do not grow", leak)
        step("10 map open/close cycles: no extra keyup listener on document", map_leak)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
