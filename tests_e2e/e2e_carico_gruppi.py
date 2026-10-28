"""Loading table with mixed files: no group lines, the rows keep the order of arrival; each row carries its group (instrument, experiment) in data-grp and an HR tag;
a DDA/IDA file is never grouped with the plain MS2 files. A loading that went well leaves no summary box."""
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
            rows = pg.evaluate("[...document.querySelectorAll('#flist tr[data-grp]')].map(r => [r.dataset.grp, r.children[1].childNodes[0].textContent.trim(), r.children[1].innerText])"); print(rows)
            assert pg.locator("#flist tr.grp").count() == 0, "no group lines: nothing appears or moves"
            assert [r[1] for r in rows] == [f.name for f in files], rows           # the order of arrival, as the rows had while loading
            keys = [r[0] for r in rows]
            assert any(k.startswith("lr:mix:") and "DDA" in k for k in keys) and any(k.startswith("lr:") and "MRM" in k for k in keys), keys
            assert all(("HR" in r[2]) == r[0].startswith("hr:") for r in rows), rows
            assert not any(k.startswith("hr:") and "ms2" in k for k in keys), keys
        step("no group lines: the rows keep their order, the group is a key and the HR tag, DDA never under plain MS2", groups)
        def quiet():
            pg.click("#opbtn"); ready(pg)
            assert pg.locator("#flist .cvsum").count() == 0
        step("a loading that went well leaves no summary box", quiet)
finally:
    r.close()
r.report()
for s in steps: print(" ", s)
