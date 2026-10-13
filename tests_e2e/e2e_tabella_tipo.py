"""Table of the loaded files: Type «standard» shows the concentration column also for a Full Scan file; Type «blank» leaves Time and concentration empty and not editable."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8874, wd="/tmp/wd_tipo")
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [{"name": n + ".mzML", "mimeType": "application/octet-stream", "buffer": open(mz(n), "rb").read()} for n in ("B_FullMass-t15", "B_FullMass-t10")]); pg.wait_for_timeout(1500)
        pg.wait_for_selector('#flist select[data-k="type"]')
        def standard():
            assert pg.locator('#flist input[data-k="conc"]').count() == 0, "no concentration column before any standard"
            pg.select_option('#flist select[data-k="type"] >> nth=0', "standard"); pg.wait_for_timeout(300)
            assert pg.locator('#flist input[data-k="conc"]:not([disabled])').count() == 1, "one editable concentration field"
            assert "oncentra" in pg.inner_text("#flist table") or "onc" in pg.inner_text("#flist table").lower()
        step("standard shows the concentration column (Full Scan file)", standard)
        def blank():
            pg.select_option('#flist select[data-k="type"] >> nth=1', "blank"); pg.wait_for_timeout(300)
            row = pg.locator("#flist tr", has=pg.locator('input[data-k="time"]')).nth(0)   # blank sorts first
            for k in ("time", "conc"):
                i = row.locator(f'input[data-k="{k}"]'); assert i.count() == 1 and i.is_disabled() and i.input_value() == "", k
        step("blank has empty and disabled Time and concentration", blank)
finally:
    pass
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n, "" if s == "ok" else s)
r.close()
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
