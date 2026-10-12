# e2e_carica_scheda: files opened one by one (a list) never leave the view on the tab of the last file; a file added later does open its tab
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
def scenario(seq):
    r = Run(port=8839, wd="/tmp/wd_carica_scheda")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            stage(pg, [mz(f) for f in FILES]); pg.click("text=Carica dati"); ready(pg)
            assert pg.evaluate("E.tab") == "full"
            # as if only the first three files had been read so far: the next step of the load adds the MS2 and MRM ones
            pg.evaluate("E.files = E.files.slice(0, 3); E.key = ''; E._seq = %s" % ("true" if seq else "false"))
            pg.evaluate("bootSession()"); pg.wait_for_function("E.files.length === 5", timeout=60000); pg.wait_for_timeout(800)
            return pg.evaluate("E.tab")
    finally:
        r.close(); r.report()
def in_list(): t = scenario(True); assert t == "full", t
def later(): t = scenario(False); assert t == "ms2", t
step("next step of a list: the tab stays Full Scan", in_list)
step("file added later: the tab of the first new file opens", later)
print("\nSTEPS"); [print(" ", s) for s in steps]
