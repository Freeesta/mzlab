"""Closing the LAST panel (one by one with the X of each panel, or with «close all») unticks every file and shows the landing; the undo opens the panels
and the ticks again, the redo closes them again."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
WORK = "document.querySelector('#v-workspace').dataset.work"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ultimo_pannello.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8899, wd="/tmp/wd_ultimo_pannello")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 950})
        pg.set_input_files("#pick", [str(EX / "FullScan_t0.mzML"), str(EX / "FullScan_t30.mzML")])
        pg.click("#opbtn"); ready(pg)
        npan = lambda: pg.evaluate("E.panels.filter(p => p.el).length")
        ticked = lambda: pg.evaluate("E.files.filter(f => f.vis && !f.gone).length")
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        def add_two():
            pg.click("#np-spec"); attendi(pg, "E.panels.filter(p => p.el).length >= 2"); assert ticked() >= 1
        step("a sample with two panels", add_two)
        def one_by_one():
            n0 = npan(); vis0 = pg.evaluate("E.files.filter(f => f.vis).map(f => f.k)")
            while npan() > 1: pg.locator(".pnl .hd .x").last.click(); ready(pg)
            assert ticked() >= 1 and pg.evaluate(WORK) == "panels", "with one panel left the files stay ticked"
            pg.locator(".pnl .hd .x").last.click(); attendi(pg, f"{WORK} === 'landing'")
            assert npan() == 0 and ticked() == 0, "the last panel closed: nothing ticked"
            assert pg.is_visible("#drop") and pg.locator("#flst .fl input:checked").count() == 0, "landing, no box checked in the side list"
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"{WORK} === 'panels'")
            assert npan() == 1 and pg.evaluate("E.files.filter(f => f.vis).map(f => f.k)") == vis0, "undo: the panel and the ticks are back"
            pg.keyboard.press(f"{mod}+Shift+z"); attendi(pg, f"{WORK} === 'landing'")
            assert npan() == 0 and ticked() == 0, "redo: closed and unticked again"
            pg.keyboard.press(f"{mod}+z"); attendi(pg, f"{WORK} === 'panels'")
        step("the X of the last panel unticks the files and shows the landing; undo and redo are coherent", one_by_one)
        def close_all():
            pg.click("#np-spec"); attendi(pg, "E.panels.filter(p => p.el).length >= 2"); n = npan(); vis0 = pg.evaluate("E.files.filter(f => f.vis).map(f => f.k)")
            pg.click("#np-closeall"); attendi(pg, f"{WORK} === 'landing'")
            assert npan() == 0 and ticked() == 0, "close all: nothing ticked"
            pg.click("#np-undo"); attendi(pg, f"{WORK} === 'panels'")
            assert npan() == n and pg.evaluate("E.files.filter(f => f.vis).map(f => f.k)") == vis0, "undo of close all: every panel and tick back"
            pg.click("#np-redo"); attendi(pg, f"{WORK} === 'landing'"); assert npan() == 0 and ticked() == 0
            pg.click("#backwork"); attendi(pg, f"{WORK} === 'panels'"); assert npan() >= 1, "«Back to work!» still brings a file back"
        step("close all: same landing; the ← and → buttons bring panels and ticks back and forth", close_all)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
