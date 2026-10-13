"""The label of a part of a mixed file names the kind of acquisition (Full Scan / DDA / MRM), not the scan level (MS1)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
r = Run(port=8851, wd="/tmp/wd_etichetta")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_viewport_size({"width": 1400, "height": 900})
        pg.set_input_files("#pick", [mz(f) for f in ("C_IDA-t0", "C_MRM-EPI-t0", "C_POLALT-t0")]); pg.wait_for_timeout(1000)
        pg.click("text=Carica dati"); ready(pg)
        lab = lambda: dict(pg.evaluate("E.files.map(f => [f.file, f.label])"))
        def labels():
            l = lab()
            assert l["C_IDA-t0.mzML#MS1"] == "C_IDA-t0 · Full Scan", l
            assert l["C_IDA-t0.mzML#MS2"] == "C_IDA-t0 · DDA", l
            assert l["C_MRM-EPI-t0.mzML#MRM"] == "C_MRM-EPI-t0 · MRM", l
            assert l["C_MRM-EPI-t0.mzML#MS2"] == "C_MRM-EPI-t0 · MS2", l
            assert l["C_POLALT-t0.mzML#MS1 pos"] == "C_POLALT-t0 · Full Scan pos", l
        step("labels name the acquisition", labels)
        def nomore():
            t = pg.evaluate("[...document.querySelectorAll('#flst .nm, #fsel option')].map(e => e.textContent).join('|')")
            assert "MS1" not in t, t
        step("no MS1 in the file list or the file menu", nomore)
        def reload():
            saved(pg); pg.reload(); pg.wait_for_timeout(1500); ready(pg)
            assert "C_IDA-t0 · Full Scan" in lab().values(), lab()
        step("restored from the notebook", reload)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad or r.errs else None
sys.exit(1 if bad else 0)
