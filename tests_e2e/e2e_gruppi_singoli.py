"""Side list, user groups that are all single files (FullScan_t0, _t5...): no header per file, rows listed flat, «Group by» hidden; a group of 2 files brings the headers and the button back
and the chosen key applies again; the key is never lost."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
D = pathlib.Path(__file__).resolve().parent / "shots" / "gruppi_singoli_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = [f"FullScan_t{t}" for t in (0, 5, 10, 15, 30)]
ds.full_scan(D / "src.mzML", 5, np.random.default_rng(2))
PATHS = []
for n in NAMES:
    q = D / f"{n}.mzML"; shutil.copyfile(D / "src.mzML", q); PATHS.append(str(q))
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_gruppi_singoli.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
HEADS = "document.querySelectorAll('#flst .fgh[data-ug]').length"
ROWS = "document.querySelectorAll('#flst .fl:not(.pr)').length"
BY = "(() => { const b = document.getElementById('sb-by'); return !b.hidden && getComputedStyle(b).display !== 'none'; })()"
def label(pg, base): return pg.evaluate("(b) => E.files.find(f => fileBase(f.file).replace(/\\.mzML$/, '') === b).ugrp || null", base)
r = Run(port=8963, wd="/tmp/wd_gruppi_singoli")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", PATHS)
        pg.click("text=Carica dati"); ready(pg, timeout=180000); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=30000)
        pg.evaluate("I18N.set('en')"); pg.wait_for_function("window.qqReady === true && I18N.lang === 'en'", timeout=60000)
        def singles():
            # one label per file, as «Group by time point» gives with five different time points
            pg.evaluate("""async () => { await carica('gruppi.js'); const m = new Map(E.files.filter(f => !f.gone).map(f => [fileBase(f.file), 'T' + f.file.match(/_t(\\d+)/)[1]]));
              for (const [b, l] of m) GRUPPI.set([b], l); }""")
            attendi(pg, "E.files.filter(f => f.ugrp).length === 5", 10000); fermo(pg, "#flst")
            assert pg.evaluate(HEADS) == 0 and pg.evaluate(ROWS) == 5, (pg.evaluate(HEADS), pg.evaluate(ROWS))
            attendi(pg, "document.getElementById('sb-by').hidden", 10000)
            assert not pg.evaluate(BY)
        step("five groups of one file: flat rows, no header, «Group by» hidden", singles)
        def pair():
            pg.evaluate("""async () => { await carica('gruppi.js'); const b = E.files.filter(f => !f.gone).map(f => fileBase(f.file)); GRUPPI.set([b[1]], 'T0'); }""")
            fermo(pg, "#flst")
            assert pg.evaluate(HEADS) == 4 and label(pg, "FullScan_t5") == "T0" and label(pg, "FullScan_t0") == "T0"
            assert pg.evaluate("document.querySelector('#flst .fgh[data-ug] em').textContent") == "2"
            attendi(pg, "!document.getElementById('sb-by').hidden", 10000); assert pg.evaluate(BY)
        step("a group of two files: the headers (4 groups) and «Group by» are back", pair)
        def keep():
            pg.evaluate("""async () => { await carica('gruppi.js'); GRUPPI.set([fileBase(E.files.find(f => f.file.includes('_t5')).file)], 'T5'); }""")
            fermo(pg, "#flst"); assert pg.evaluate(HEADS) == 0
            assert pg.evaluate("E.ugBy && E.ugBy.by") == "custom" and label(pg, "FullScan_t5") == "T5", "the chosen key or labels were lost"
            pg.evaluate("""async () => { await carica('gruppi.js'); GRUPPI.set([fileBase(E.files.find(f => f.file.includes('_t10')).file)], 'T5'); }""")
            fermo(pg, "#flst"); assert pg.evaluate(HEADS) == 4
        step("the labels stay on the files: the headers come back with the next group of two", keep)
        def miss():
            assert not pg.evaluate("window.MZLAB_LR_MISS || []"), pg.evaluate("window.MZLAB_LR_MISS")
        step("no engine miss", miss)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
shutil.rmtree(D, ignore_errors=True)
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
sys.exit(1 if any(s != "ok" for _, s in steps) else 0)
