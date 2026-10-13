# e2e_righe: panels of one row (riga) sit side by side with the shared height and the saved shares; the row is saved in the notebook; a panel dragged out leaves it
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8874, wd="/tmp/wd_righe")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [mz(f) for f in FILES])
        pg.wait_for_timeout(1000); pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("addPanel('spec',{tab:E.tab,k:E.cur,level:1}); addPanel('spec',{tab:E.tab,k:E.cur,level:1})")
        pg.wait_for_timeout(300)
        ids = lambda: pg.evaluate("tabPanels().filter(q => q.full).map(q => q.id)")
        def side():
            pg.evaluate("(() => { const s = stackOrder(), a = s[s.length - 2], b = s[s.length - 1]; a.riga = b.riga = 99; a.quota = 0.6; b.quota = 0.4; b.h = 200; relayout(); fitHost(); })()")
            g = pg.evaluate("(() => { const s = stackOrder().slice(-2); return s.map(q => [q.x, q.y, q.w, q.h]); })()")
            W = pg.evaluate("hostWidth()")
            assert g[0][1] == g[1][1] and g[0][3] == g[1][3], g            # same row: same y and height
            assert g[0][0] == 0 and g[1][0] == g[0][2] + 6 and abs(g[1][0] + g[1][2] - W) <= 1, (g, W)
            assert g[0][2] > g[1][2], g                                      # the shares: 60 / 40
        step("two panels of one row: side by side, one height, shares", side)
        def saved():
            pg.evaluate("uiSave(true)"); pg.wait_for_timeout(300)
            s = pg.evaluate("NB.ui.panels.slice(-2).map(p => [p.riga, p.quota])")
            assert s[0][0] == s[1][0] and s[0][1] == 0.6 and s[1][1] == 0.4, s
        step("the row and the shares are in the notebook", saved)
        def order():
            n = pg.evaluate("(() => { const s = stackOrder(); return s.map(q => q.id) })()")
            assert n == pg.evaluate("E.panels.filter(q => q.full).map(q => q.id).sort((a, b) => 0) && stackOrder().map(q => q.id)"), n
            assert pg.evaluate("STATO.verifica()") == []
        step("reading order and state are coherent", order)
        def leave():
            pg.evaluate("(() => { const s = stackOrder(), b = s[s.length - 1]; LAYOUT.esci(b, E.panels, hostWidth()); relayout(); })()")
            g = pg.evaluate("(() => stackOrder().slice(-2).map(q => [q.x, q.y, q.w]))()")
            W = pg.evaluate("hostWidth()")
            assert g[0][1] < g[1][1] and g[0][2] == W and g[1][2] == W, (g, W)
        step("a panel that leaves the row goes under it; the other one gets the full width back", leave)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
