"""Marked points in high resolution (synthetic Orbitrap file): the points of one group of RT (apexes within the window, default ±0.02 min) share a shape
(circle, square, triangle, diamond, by turns), the table puts the members of a group together under a row that can be folded, the option «snap» slides a point
onto the apex of the peak (api/mapsnap) and, switched off, keeps it where it was put."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from lib import *
from lib_hr import synth, load
import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_mappa_gruppi.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
M = "E.panels.find(q=>q.type==='map')"
D = synth("mappa_gruppi")
PARENT = ds.hr_parent_mz()
r = Run(port=8897, wd="/tmp/wd_mappa_gruppi")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"], 4000, 2)
        pg.evaluate("document.querySelectorAll('#tour-invito button').forEach(b => b.textContent.trim() === 'Not now' && b.click())")
        pg.evaluate("setTab('full',true)"); pg.wait_for_timeout(500)
        pg.evaluate("reveal(addPanel('map', { k: E.files.find(f => f.kind === 'full').k }))"); ready(pg); pg.wait_for_timeout(1500)
        pg.evaluate(M + ".el.scrollIntoView({block:'start'})"); pg.wait_for_timeout(300)
        add = lambda rt, mz: pg.evaluate(f"MAPPA.addPointAt({M}, {rt}, {mz}, 0.01)")
        rows = lambda: pg.evaluate(f"({M}._mz.rows || []).map(r => r.g)")
        def settle(n):
            pg.wait_for_function(f"({M}._mz.rows || []).length === {n} && !!document.querySelector('.pnl.map .mztab')", timeout=15000); pg.wait_for_timeout(300)
        def groups():
            add(9.0, 305.0702); add(14.3, PARENT); add(14.31, PARENT + 1.00335); settle(3)
            g = rows(); assert g[1] == g[2] and g[0] != g[1], g                              # the two at 14.3 are one compound, the one at 9 another
            sh = pg.evaluate(f"[0, 1, 2].map(i => MAPPA.grpShape({M}, i))")
            assert sh[1] == sh[2] and sh[0] != sh[1] and set(sh) <= {"circle", "square", "triangle", "diamond"}, sh   # shape by group, not by colour
        step("HR: points of one group of RT share a shape, the others have another", groups)
        def table():
            assert pg.locator(".pnl.map .mztab tr.mzgr").count() == 1                         # one row of the group (the lone point has none)
            assert pg.locator(".pnl.map .mztab tbody tr[data-i]").count() == 3
            txt = pg.inner_text(".pnl.map .mztab tr.mzgr"); assert "2, 3" in txt, txt         # its members, side by side
            pg.click(".pnl.map .mztab [data-gtog]"); pg.wait_for_timeout(300)
            assert pg.locator(".pnl.map .mztab tbody tr[data-i]").count() == 1                # folded: only the lone point stays
            assert pg.get_attribute(".pnl.map .mztab [data-gtog]", "aria-expanded") == "false"
            pg.click(".pnl.map .mztab [data-gtog]"); pg.wait_for_timeout(300)
            assert pg.locator(".pnl.map .mztab tbody tr[data-i]").count() == 3
            assert "■" in pg.inner_text(".pnl.map .mztab tbody") or "▲" in pg.inner_text(".pnl.map .mztab tbody") or "◆" in pg.inner_text(".pnl.map .mztab tbody")
        step("HR: the table puts the members of a group together and the group row folds", table)
        def snap():
            k = pg.evaluate(M + "._a.f.k")
            at = pg.evaluate(f"fetch('api/mapsnap?k={k}&level=1&rt=14.33&mz={PARENT + 0.004}&dmz=0.01').then(r => r.json())")
            assert at.get("rt") is not None, at
            n0 = len(pg.evaluate(f"MAPPA.pts({M})")); add(14.33, PARENT + 0.004); settle(n0 + 1)
            last = pg.evaluate(f"MAPPA.pts({M}).slice(-1)[0]"); assert abs(last["rt"] - at["rt"]) < 1e-9 and abs(last["mz"] - at["mz"]) < 1e-9, (last, at)
            assert pg.get_attribute(".pnl.map [data-mz=snap]", "aria-pressed") == "true"
            pg.click(".pnl.map [data-mz=snap]"); assert pg.get_attribute(".pnl.map [data-mz=snap]", "aria-pressed") == "false"
            add(14.33, PARENT + 0.004); settle(n0 + 2)
            last = pg.evaluate(f"MAPPA.pts({M}).slice(-1)[0]"); assert last["rt"] == 14.33 and abs(last["mz"] - (PARENT + 0.004)) < 1e-9, last     # off: where it was put
            pg.click(".pnl.map [data-mz=snap]")
        step("snap: on, the point slides onto the apex of api/mapsnap; off, it stays", snap)
        def align():
            pg.click(".pnl.map [data-mz=snap]")                                               # snap off: the points stay where they are put
            n0 = len(pg.evaluate(f"MAPPA.pts({M})")); pg.evaluate("MAPPA.prefs.grp = 0.1")
            add(20.0, 400.0); add(20.07, 410.0); add(20.1, 420.0); settle(n0 + 3)
            g = rows()[-3:]; assert g[0] == g[1] == g[2], g                                   # a wider window groups apexes a few scans apart
            assert pg.is_visible(".pnl.map .mztab [data-t=align]")
            pg.check(".pnl.map .mztab [data-t=align]"); attendi(pg, f"MAPPA.pts({M}).slice(-3).every(q => q.rt === 20)"); settle(n0 + 3)
            rts = pg.evaluate(f"MAPPA.pts({M}).slice(-3).map(q => q.rt)"); assert rts == [20.0, 20.0, 20.0], rts   # all on the RT of the first one
            pg.uncheck(".pnl.map .mztab [data-t=align]"); pg.click(".pnl.map [data-mz=snap]")
        step("align: points of a group go to the RT of the first one", align)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
