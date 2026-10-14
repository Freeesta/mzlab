# e2e_apri_uno: a mixed low-resolution load (Full Scan + MS2 + MRM) opens only the first file of the list; the others stay in the list and a click opens them
import os, sys; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
r = Run(port=8845, wd="/tmp/wd_apri_uno")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [mz(f) for f in FILES]); pg.click("text=Carica dati"); ready(pg)
        def only_first():
            v = pg.evaluate("E.files.map(f=>[f.file,f.kind,f.vis])")
            assert len(v) == 5 and len({x[1] for x in v}) == 3, v
            assert [x[0] for x in v if x[2]] == [v[0][0]], v
            attendi(pg, "E.panels.length > 0"); assert pg.evaluate("E.panels.every(p=>p.tab==='full')"), pg.evaluate("E.panels.map(p=>p.tab)")      # no block for the closed types
        step("Full Scan + MS2 + MRM: only the first file is open", only_first)
        def click_opens():
            if not pg.is_visible("#flst"): pg.click("#stbar [data-cmd=file]")
            pg.wait_for_selector("#flst .fl", state="visible"); pg.click("#flst .fl:has-text('MRM')")
            attendi(pg, "E.files.filter(f=>f.vis).length === 2 && E.files.some(f=>f.vis&&f.kind==='mrm') && E.panels.some(p=>p.tab==='mrm')")
        step("a click on a closed file opens it", click_opens)
finally:
    r.close(); r.report()
print("\nSTEPS"); [print(" ", s) for s in steps]
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
