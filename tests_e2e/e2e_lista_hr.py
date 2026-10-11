"""HR file list: only the first file (in list order) is open, groups DDA / Full Scan / MS2 / Infusion, a click opens another file, compact list folds groups."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import synth, load, hr_variants
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_lista_hr.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = synth("hrlista")
V = hr_variants(D / "HR_DDA-Exploris-t30.mzML", D)
r = Run(port=8923, wd="/tmp/wdhrlista")
GROUPS = "Array.from(document.querySelectorAll('#flst .fgh')).map(g=>g.querySelector('span:not(.chv)').textContent+'|'+g.querySelector('em').textContent)"
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, [D / "HR_DDA-Exploris-t30.mzML"] + V, 6000, 5); pg.wait_for_timeout(1500)
        def first_only():
            v = pg.evaluate("E.files.map(f=>[f.file,f.vis])")
            assert [x[0] for x in v if x[1]] and all(x[0].startswith("HR_full-t5") for x in v if x[1]), v
            assert sum(1 for x in v if not x[1]) == 4, v
        step("only the first file (by time) is open", first_only)
        def groups():
            g = pg.evaluate(GROUPS)
            assert sorted(g) == sorted(["DDA|1", "Full Scan|1", "MS2 (Product Ion)|1", "Infusione|1"]), g
        step("groups DDA (one row for both parts), Full Scan, MS2, Infusione", groups)
        def click_opens():
            if not pg.is_visible("#flst"): pg.click("#stbar [data-cmd=file]"); pg.wait_for_timeout(500)       # the side bar starts folded
            pg.click("#flst .fl:has-text('HR_inf')"); pg.wait_for_timeout(500)
            assert pg.evaluate("E.files.filter(f=>f.vis).map(f=>f.file.split('.')[0]).sort().join()") == "HR_full-t5,HR_inf-t15"
            pg.click("#flst .fl:has-text('HR_DDA') input[data-k]"); pg.wait_for_timeout(500)
            assert pg.evaluate("E.files.filter(f=>f.vis&&f.file.startsWith('HR_DDA')).length") == 2
        step("a click on a closed file opens it; the DDA check box opens both parts", click_opens)
        def compact():
            pg.set_viewport_size({"width": 1000, "height": 1200}); pg.wait_for_timeout(300); pg.evaluate("renderFileList()"); drawer(pg, pin=False)         # narrow window: the drawer only opens on demand
            assert pg.locator("#flst .fgh[data-gg]").count() == 4
            pg.click("#flst .fgh[data-gg='hr.dda']"); pg.wait_for_timeout(200)
            assert pg.evaluate("JSON.parse(localStorage.getItem('qqq.flgrp'))['hr.dda']") == 1
            assert not pg.is_visible("#flst .fl:has-text('HR_DDA')")
            pg.evaluate("renderFileList()"); assert pg.get_attribute("#flst .fgh[data-gg='hr.dda']", "aria-expanded") == "false"
            pg.set_viewport_size({"width": 1500, "height": 2200}); pg.wait_for_timeout(300); pg.evaluate("renderFileList()"); drawer(pg)
            assert pg.locator("#flst .fgh[data-gg]").count() == 0 and pg.is_visible("#flst .fl:has-text('HR_DDA')")
        step("narrow list: groups fold with memory; wide window unchanged", compact)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
