"""The «Groups» control of the side panel header: named «Gruppi»/«Groups», always fully visible (no ellipsis, not in the overflow of the header), and present only while the file list is the selected panel."""
import sys, os, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
D = pathlib.Path(__file__).resolve().parent / "shots" / "gruppi_visibile_tmp"; D.mkdir(parents=True, exist_ok=True)
NAMES = ["MQ_blk", "MQ_Wash_1", "TW_blk", "TW_Wash_1"] + [f"MQ_{c}_{t}" for c in ("H2O2", "N2") for t in ("T00", "T01")]
ds.full_scan(D / "src.mzML", 5, np.random.default_rng(2))
PATHS = []
for n in NAMES:
    q = D / f"{n}.mzML"; shutil.copyfile(D / "src.mzML", q); PATHS.append(str(q))
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
r = Run(port=8963, wd="/tmp/wd_gruppi_vis")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", PATHS); pg.click("text=Carica dati"); ready(pg, timeout=180000); drawer(pg)
        pg.wait_for_selector("#flst .fl .nm", timeout=30000)
        attendi(pg, "BARRA.curTab === 'file' && !document.querySelector('#sb-by').hidden", 20000)
        def full():
            assert pg.inner_text("#sb-by").strip() == "Gruppi", pg.inner_text("#sb-by")
            w = pg.evaluate("(() => { const b = document.querySelector('#sb-by'), s = b.querySelector('span'); return [b.scrollWidth - b.clientWidth, s.scrollWidth - s.clientWidth]; })()")
            assert w[0] <= 0 and w[1] <= 0, f"truncated: {w}"
            assert pg.is_visible("#sb-by") and pg.evaluate("!document.querySelector('#tb-more') || !(document.querySelector('#tb-more').dataset.dropped || '').includes('by')")
        step("file list: «Gruppi» is fully visible", full)
        def only_files():
            for t in ("calc", "adducts", "losses", "lists"):
                pg.evaluate(f"BARRA.setTab('{t}')"); attendi(pg, f"BARRA.curTab === '{t}'")
                assert not pg.is_visible("#sb-by"), f"«Gruppi» shown in the {t} panel"
            pg.evaluate("BARRA.setTab('file')"); attendi(pg, "BARRA.curTab === 'file'")
            assert pg.is_visible("#sb-by"), "back on the file list it is there again"
        step("hidden in Calculator / Adducts / Losses / Lists, back on the file list", only_files)
        def english():
            pg.evaluate("I18N.set('en')"); pg.wait_for_function("I18N.lang === 'en'", timeout=60000)
            assert pg.inner_text("#sb-by").strip() == "Groups", pg.inner_text("#sb-by")
        step("English: «Groups»", english)
finally:
    r.close(); shutil.rmtree(D, ignore_errors=True)
r.report()
for s in steps: print(" ", s)
