"""MS2 transitions (low resolution): the chromatogram panel of the MS2 tab lists the precursor > fragment pairs, like the MRM panel; choosing one redraws the chromatogram."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_ms2_transizioni.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
PNL = ".pnl:has(select[data-o=tr])"
r = Run(port=8992, wd="/tmp/wd_ms2tr")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 1000})
        pg.set_input_files("#pick", [mz("B_MS2-t15")])
        pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=30000)
        pg.click("text=Carica dati"); ready(pg)
        pg.click("#dtabs [data-t=ms2]"); pg.wait_for_timeout(1500)
        def listed():
            pg.wait_for_selector("select[data-o=tr]", timeout=15000)
            n = pg.evaluate("document.querySelectorAll('select[data-o=tr] option').length"); assert n >= 4, n          # none, all, and the pairs
        step("the MS2 chromatogram lists the transitions", listed)
        def pick():
            pg.select_option("select[data-o=tr]", index=2); pg.wait_for_timeout(1500)
            s = pg.evaluate("E.panels.filter(p=>p.type==='chrom'&&p.tr).map(p=>p._a.sr.map(s=>s.ion))")
            assert s and len(s[0]) == 1 and ">" in s[0][0], s
        step("choosing one gives the chromatogram of that pair", pick)
        def allt():
            pg.select_option("select[data-o=tr]", "__all"); pg.wait_for_timeout(1500)
            s = pg.evaluate("E.panels.filter(p=>p.type==='chrom'&&p.tr).map(p=>p._a.sr.length)"); assert s and s[0] > 1, s
        step("all the transitions are overlaid", allt)
        def back():
            pg.select_option("select[data-o=tr]", ""); pg.wait_for_timeout(1000)
            s = pg.evaluate("E.panels.filter(p=>p.type==='chrom').map(p=>p._a.sr.map(s=>s.ion))"); assert s[0] == ["TIC"], s
        step("none: the TIC of the MS2 scans as before", back)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
