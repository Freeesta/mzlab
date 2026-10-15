"""MS2 entry of the file list (demo files): the tick opens ONLY the precursor chromatogram, with the MS2 file made visible (it was empty: «no file visible»); the double click on the name opens the product-ion spectrum."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8896, wd="/tmp/wd_ms2voce")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / f"FullScan_t{t}.mzML") for t in (0, 5, 10, 15, 30)] + [str(ES / f"MS2_t{t}.mzML") for t in (15, 60)])
        pg.click("text=Carica dati"); ready(pg)
        P = "E.panels.filter(p => p.tab === 'ms2')"
        def tick():
            assert pg.evaluate("E.files.find(f => f.label === 'MS2_t15').vis") is False
            pg.locator("#flst input[data-pr]").first.click(); pg.wait_for_timeout(2500); ready(pg)
            t = pg.evaluate(f"{P}.map(p => p.type)"); assert t == ["chrom"], t
            assert pg.evaluate("E.files.find(f => f.label === 'MS2_t15').vis") is True, "the MS2 file of the precursor is visible"
            n = pg.evaluate(f"(async () => (await rawSeries({P}[0])).filter(s => s.y.some(v => v > 0)).length)()"); assert n >= 1, "the precursor chromatogram has data"
            assert pg.locator("#flst input[data-pr]:checked").count() == 1
        step("tick: only the precursor chromatogram, MS2 file visible, with data", tick)
        def dbl():
            pg.locator("#flst [data-pg]").first.dblclick(); pg.wait_for_timeout(2500); ready(pg)
            t = pg.evaluate(f"{P}.map(p => p.type)"); assert t == ["chrom", "spec"], t
            s = pg.evaluate(f"(() => {{ const s = {P}[1], c = {P}[0]; return [s.level, String(s.prec), s.link === c.id, s.r0 != null]; }})()"); assert s == [2, "229.1", True, True], s
        step("double click on the name: the product-ion spectrum, linked", dbl)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
