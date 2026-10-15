"""Second load (demo files): the panels already open stay on their files, with data, when a second group of files is loaded (the list is reopened file by file; the files not back yet keep their index by name)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
def specs(pg): return "E.panels.filter(p => p.tab === 'ms2' && p.type === 'spec')"
r = Run(port=8898, wd="/tmp/wd_ms2ric")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / f"MS2_t{t}.mzML") for t in (15, 60)])
        pg.click("text=Carica dati"); ready(pg)
        pg.locator('#flst [data-pg="229.1"]').dblclick(); attendi(pg, f"{specs(pg)}.length >= 1"); ready(pg)
        def second():
            pg.click("#addf"); pg.set_input_files("#pick", [str(ES / f"FullScan_t{t}.mzML") for t in (0, 5)])
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length >= 2")
            pg.click("#opbtn"); attendi(pg, "E.files.length === 4 && !(E.pending || []).length"); ready(pg)
            q = pg.evaluate(f"{specs(pg)}.map(p => [E.files[p.k].label, p._a && p._a.data ? p._a.data.length : 0])")
            assert len(q) == 1 and q[0][0] == "MS2_t15" and q[0][1] >= 1, q
            assert pg.evaluate("E.files[E.cur].label") == "MS2_t15", "the active file follows its name"
        step("second load: MS2 panels stay on their files and keep data", second)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
