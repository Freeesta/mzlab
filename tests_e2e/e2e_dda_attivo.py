"""Four HR DDA files (blank, t0, t15, t30): the file open first is not the blank, the spectrum follows the file picked in the list, no '· Full Scan' on a DDA."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from lib_hr import load, HERE, ds, np
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_dda_attivo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = HERE / "shots" / "ddaattivo_tmp"; D.mkdir(parents=True, exist_ok=True)
files = []
for i, n in enumerate(["LC_POS_blank", "LC_POS_t0", "LC_POS_t15", "LC_POS_t30"]):
    ds.hr_dda(D / (n + ".mzML"), np.random.default_rng(5 + i), "exploris"); files.append(D / (n + ".mzML"))
r = Run(port=8924, wd="/tmp/wdddaattivo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        load(pg, files, 6000, 8)
        pg.wait_for_function("E.files.filter(f => f.vis).length === 2 && !E.files[0].vis", timeout=30000)
        def first_not_blank():
            assert pg.evaluate("E.files[E.cur].file.startsWith('LC_POS_t0')"), pg.evaluate("E.files[E.cur].file")
            assert pg.evaluate("E.files.filter(f => f.vis).every(f => f.file.startsWith('LC_POS_t0'))")
            assert pg.evaluate("E.panels.find(p => p.type === 'spec' && p.link != null).k") == 2
        step("the file open first is the first that is not a blank; the list order is unchanged", first_not_blank)
        def follows():
            pg.click("#flst .fl:has-text('t15') .nm")
            pg.wait_for_function("E.panels.find(p => p.type === 'spec' && p.link != null).k === 4", timeout=10000)
            pg.click("#flst .fl:has-text('t30') .nm")
            pg.wait_for_function("E.panels.find(p => p.type === 'spec' && p.link != null).k === 6", timeout=10000)
        step("the live spectrum follows the file picked in the list", follows)
        def no_full_scan():
            assert pg.evaluate("E.files.filter(f => f.dda).every(f => !/Full Scan/.test(f.label))")
            t = pg.evaluate("[...document.querySelectorAll('.pnl')].map(e => e.innerText).join(' ')")
            assert "Full Scan" not in t, t[:300]
        step("no '· Full Scan' on a DDA file in the legend or the selectors", no_full_scan)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
