"""An index written by a newer program (header version above the known one) in OPFS: the file is uploaded again, the page warns (error.newerFormat), the index is not
touched and not rebuilt. An index of the known version is replaced as before. Synthetic file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_formato_nuovo.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:300]))
import numpy as np
sys.path.insert(0, str(ROOT / "tools"))
import dati_sintetici as ds
D = Path("/tmp/nuovo_e2e"); D.mkdir(exist_ok=True)
F = D / "C_FullMass-t0.mzML"; ds.full_scan(F, 0, np.random.default_rng(10))
IDX = "C_FullMass-t0.mzML.mzidx"
HASH = """async () => { const r = await navigator.storage.getDirectory(); const h = await (await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle('idx')).getFileHandle('%s'); const b = new Uint8Array(await (await h.getFile()).arrayBuffer()); const x = await crypto.subtle.digest('SHA-256', b); return Array.from(new Uint8Array(x)).map(v => v.toString(16).padStart(2, '0')).join(''); }""" % IDX
VERSION = """async v => { const r = await navigator.storage.getDirectory(); const h = await (await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle('idx')).getFileHandle('%s'); const w = await h.createWritable({ keepExistingData: true }); await w.write({ type: 'write', position: 8, data: new Uint8Array([v, 0, 0, 0]) }); await w.close(); }""" % IDX
r = Run(port=8995, wd="/tmp/wdnuovo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)]); attendi(pg, "document.body.innerText.includes('Carica dati')")
        pg.click("text=Carica dati"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        def again():
            pg.goto(pg.url); attendi(pg, "document.querySelector('#pick')")
            pg.set_input_files("#pick", [str(F)]); attendi(pg, "document.body.innerText.includes('C_FullMass-t0.mzML')")
        def newer():
            before = pg.evaluate(HASH); pg.evaluate(VERSION, 3); patched = pg.evaluate(HASH); assert patched != before
            again()
            attendi(pg, "/formato 3/.test(document.body.innerText)", timeout=60000)
            assert pg.evaluate(HASH) == patched, "the index of the newer program was not touched"
            assert pg.evaluate("window.MZLAB_LR_MISS || []") == [], "no route unavailable"
        step("a newer index: warning, index untouched", newer)
        def known():
            pg.evaluate(VERSION, 1); again()
            attendi(pg, "document.querySelector('#opbtn') && !document.querySelector('#opbtn').disabled", timeout=60000)
            assert "formato 3" not in pg.evaluate("document.body.innerText"), "no warning for a known version"
        step("a known version: no warning", known)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
