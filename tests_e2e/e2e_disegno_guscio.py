"""Disegno shell first: on a direct #/draw address the loading note and the "Quick draw" field are shown before Ketcher is ready, and a SMILES sent in that gap is drawn once the editor is up."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
r = Run(port=8934, wd="/tmp/wd_dguscio")
fail = []
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1300, "height": 900})
        pg.evaluate("() => { location.hash = '#/draw'; }")
        attendi(pg, "!!document.getElementById('ex-load') && document.getElementById('ex-load').onclick !== null && !document.getElementById('kload').hidden", timeout=30000)
        early = pg.evaluate("() => ({ready: !!(window.TPDraw && TPDraw.ready()), smi: getComputedStyle(document.getElementById('ex-smi')).display !== 'none', note: !document.getElementById('kload').hidden})")
        if early["ready"]: fail.append("Ketcher was ready before the shell could be checked: " + str(early))
        if not (early["smi"] and early["note"]): fail.append("shell not shown while the editor loads: " + str(early))
        pg.fill("#ex-smi", "c1ccccc1"); pg.click("#ex-load")
        attendi(pg, "document.getElementById('kframe').classList.contains('kwait') === false && !!(window.TPDraw && TPDraw.ready())", timeout=90000)
        attendi(pg, "document.getElementById('ex-smi').value === ''", timeout=30000)
        smi = pg.evaluate("async () => await TPDraw.smiles()")
        if "c1ccccc1" not in str(smi) and "C1=CC=CC=C1" not in str(smi): fail.append("the early SMILES was not drawn: " + str(smi))
finally:
    r.close()
print("FAIL: " + "; ".join(fail) if fail else "OK")
sys.exit(1 if fail else 0)
