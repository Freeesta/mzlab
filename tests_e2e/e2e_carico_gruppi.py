"""Loading table with mixed files: the groups follow the instrument first (low resolution, then high resolution), then the experiment;
a DDA/IDA file is never listed with the plain MS2 files. A loading that went well leaves no summary box."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
D = pathlib.Path(__file__).resolve().parent / "shots" / "gruppi_tmp"; D.mkdir(parents=True, exist_ok=True)
ds.mixed_ida(D / "C_IDA-t0.mzML", np.random.default_rng(1)); ds.mixed_mrm_epi(D / "C_MRM-EPI-t0.mzML", np.random.default_rng(2))
hr = True
r = Run(port=8883, wd="/tmp/wd83")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        files = [D / "C_IDA-t0.mzML", D / "C_MRM-EPI-t0.mzML"]
        ds.hr_dda(D / "HR_DDA-Exploris-t0.mzML", np.random.default_rng(3)); files.append(D / "HR_DDA-Exploris-t0.mzML")
        stage(pg, files)
        def groups():
            g = pg.locator("#flist tr.grp").all_inner_texts(); print(g)
            low = [x for x in g if "HR" not in x]
            assert any("Full Scan + DDA" in x for x in low) and any("MRM" in x for x in g), g
            if hr:
                i = [k for k, x in enumerate(g) if "HR" in x]
                assert i and i[0] > max(k for k, x in enumerate(g) if "HR" not in x), g
                assert not any("MS2 (Product Ion)" in x and "HR" in x for x in g), g
        step("groups: low resolution first, high resolution apart, DDA never under plain MS2", groups)
        def quiet():
            pg.click("#opbtn"); ready(pg)
            assert pg.locator("#flist .cvsum").count() == 0
        step("a loading that went well leaves no summary box", quiet)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
