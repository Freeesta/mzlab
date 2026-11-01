"""After start-up, adding files never raises the full-screen loading veil (#loading): files of another experiment type arriving while one is open
(MRM and MS2 after a Full Scan) are read in the background, the page stays clickable and a tab opened for the first time shows no veil either."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
lib.AUTO_TEMPI = False
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, str(e).split("\n")[0][:240]))
r = Run(port=8830 + 161, wd="/tmp/wd_nessun_velo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        stage(pg, [str(EX / "FullScan_t0.mzML")])
        open_first(pg)
        ready(pg)
        pg.evaluate("window.__veil = false; new MutationObserver(() => { if (!document.querySelector('#loading').hidden) window.__veil = true; }).observe(document.querySelector('#loading'), { attributes: true, attributeFilter: ['hidden'] })")
        def more_files():
            pg.set_input_files("#pick", [str(EX / "MRM_std_0.6ppm.mzML"), str(EX / "MS2_t15.mzML"), str(EX / "FullScan_t5.mzML")])
            # while the files are read nothing covers the page: the point at its centre is not the veil
            assert pg.evaluate("document.querySelector('#loading').hidden && !document.elementFromPoint(innerWidth / 2, innerHeight / 2)?.closest('#loading')"), "the veil covers the page"
            loaded(pg)
            ready(pg)
            assert pg.evaluate("window.__veil") is False, "the veil was shown while files were added"
        step("files of other types added while one is open: no veil", more_files)
        def tabs_no_veil():
            for k in ("mrm", "ms2", "full"):
                pg.evaluate(f"setTab('{k}')")
                attendi(pg, f"E.tab === '{k}'")
                ready(pg)
            assert pg.evaluate("window.__veil") is False, "the veil was shown by a tab"
        step("opening a tab for the first time: no veil", tabs_no_veil)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("OK   " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else "  -> " + s))
errs = [e for e in r.errs if e[0] in ("pageerror",)]
print("page errors:", errs)
sys.exit(1 if bad or errs else 0)
