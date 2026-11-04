"""Loading table: the sample name has no file extension (the cell title and the open button's label keep the real file name)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
ES = ROOT / "mzlab" / "web" / "esempi"
r = Run(port=8904, wd="/tmp/wd_tabella_nomi")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, [str(ES / "FullScan_t0.mzML"), str(ES / "MS2_t15.mzML")])
        pg.click("text=Carica dati"); ready(pg)
        pg.evaluate("carica('colonne-campioni.js').then(() => renderFiles())"); pg.wait_for_function("!!window.COLS")
        pg.evaluate("renderFiles()")
        got = pg.evaluate("[...document.querySelectorAll('#flist .ldname')].map(x => [x.textContent.trim(), x.title])")
        assert sorted(g[0] for g in got) == ["FullScan_t0", "MS2_t15"], got
        assert sorted(g[1] for g in got) == ["FullScan_t0.mzML", "MS2_t15.mzML"], got
        print("ok")
finally:
    r.close()
