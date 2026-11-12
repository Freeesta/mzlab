"""The summary of a loading (CONVROWS.summary: partial reads, failures) goes with the files it describes: «Remove all files» leaves no box and raises
none, removing one file drops only its line (the box with its last line), «Close» still clears it and a new loading shows a fresh one.
Synthetic files only: one whole mzML and two cut short (a partial read each)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import misura_lista_molti as mlm
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_togli_resoconto.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
BOX = "#flist .cvsum"
MISS = BOX + " li[data-cvmiss]"
r = Run(port=8973, wd="/tmp/wd_togli_resoconto")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.wait_for_selector("#drop"); wait_for_js(pg, "window.qqReady === true", 60000)
        src = Path("/tmp/wd_togli_resoconto_in"); base = mlm.small_files(src, 3)
        cut = src / "cut"; cut.mkdir(exist_ok=True); files = []
        for i, f in enumerate(base):      # the first whole, the other two cut after 60 % of their bytes
            b = Path(f).read_bytes(); out = cut / Path(f).name
            out.write_bytes(b if i == 0 else b[:len(b) * 6 // 10]); files.append(str(out))
        names = [Path(f).name for f in files]
        def load():
            pg.set_input_files("#pick", files)
            wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && CONVROWS.running().length === 0", 120000)
            if pg.is_hidden("#wtable"): pg.click("#samptab")      # the sample table, where the user sees the box
            pg.wait_for_selector(MISS, timeout=30000)
            hold(pg, 300)
        def no_box(why):
            hold(pg, 600)       # past the repaint of the removal and of the queue
            assert pg.locator(BOX).count() == 0, f"{why}: " + pg.inner_text(BOX)
        def remove_all():
            load()
            assert pg.locator(MISS).count() == 2, pg.inner_text(BOX)
            pg.click("#rmall")
            pg.click("#askok", timeout=5000)
            wait_for_js(pg, "E.files.filter(f => !f.gone).length === 0", 30000)
            no_box("box left after Remove all")
            wait_for_js(pg, "ST.files.length === 0", 30000)       # the window of Undo closes and the rows are freed
            no_box("box raised after the rows were freed")
        step("Remove all files: the summary box goes with them and none comes back when they are freed", remove_all)
        def remove_one():
            load()
            cutname = names[1]
            assert pg.locator(f'{MISS}[data-cvrow="{cutname}"]').count() == 1, pg.inner_text(BOX)
            pg.evaluate(f"removeFile(E.files.find(f => !f.gone && (f.file || '').split('#')[0] === {cutname!r}))")
            hold(pg, 400)
            assert pg.locator(f'{BOX} li[data-cvrow="{cutname}"]').count() == 0, pg.inner_text(BOX)
            assert pg.locator(MISS).count() == 1 and pg.locator(BOX + " li").count() == 1, pg.inner_text(BOX)
            pg.evaluate(f"removeFile(E.files.find(f => !f.gone && (f.file || '').split('#')[0] === {names[0]!r}))")
            hold(pg, 400)
            assert pg.locator(MISS).count() == 1, "removing a file with no line touched the others: " + pg.inner_text(BOX)
            pg.evaluate(f"removeFile(E.files.find(f => !f.gone && (f.file || '').split('#')[0] === {names[2]!r}))")
            no_box("box left after its last line went")
        step("removing one file drops only its line, the box goes with its last line", remove_one)
        def fresh():
            pg.evaluate("removeAllFiles()"); wait_for_js(pg, "ST.files.length === 0", 30000); no_box("box after Remove all")
            load()
            assert pg.locator(MISS).count() == 2 and pg.locator(BOX + " li").count() == 2, pg.inner_text(BOX)
            pg.click(BOX + " [data-cvact=close]"); no_box("box left after Close")
            pg.evaluate("removeAllFiles()"); wait_for_js(pg, "ST.files.length === 0", 30000); load()
            assert pg.locator(MISS).count() == 2, pg.inner_text(BOX)
            pg.evaluate("removeAllFiles()"); no_box("box after the second Remove all")
        step("a new loading shows a fresh summary, «Close» clears it, Remove all clears it again", fresh)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
