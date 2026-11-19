"""An index written by a newer program (header version above the known one) in OPFS: the file is uploaded again, the page warns (error.newerFormat), the index is not
touched and not rebuilt. An index of the known version is replaced as before. Synthetic file."""
# covers: mzlab/web/manifesto.js, mzlab/web/archivio.js, mzlab/web/rust-worker.js
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
VERSION = """async v => { const r = await navigator.storage.getDirectory(); const h = await (await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle('idx')).getFileHandle('%s'); let w; for (let i = 0; !w; i++) { try { w = await h.createWritable({ keepExistingData: true }); } catch (e) { if (i > 100) throw e; await new Promise(ok => setTimeout(ok, 100)); } } await w.write({ type: 'write', position: 8, data: new Uint8Array([v, 0, 0, 0]) }); await w.close(); }""" % IDX
HEADER = """async () => { const r = await navigator.storage.getDirectory(); const h = await (await (await r.getDirectoryHandle('mzlab')).getDirectoryHandle('idx')).getFileHandle('%s'); const b = new DataView(await (await h.getFile()).arrayBuffer()); return b.getUint32(8, true); }""" % IDX
r = Run(port=8995, wd="/tmp/wdnuovo")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.set_input_files("#pick", [str(F)])
        pg.click("text=Load data"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
        def again():
            pg.goto(pg.url); wait_for_js(pg, "document.querySelector('#pick')")
            pg.set_input_files("#pick", [str(F)]); wait_for_js(pg, "(UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.some(f => !f.gone)) || /format 4/.test(document.body.innerText)", timeout=60000)
        def newer():
            before = pg.evaluate(HASH); pg.evaluate(VERSION, 4); patched = pg.evaluate(HASH); assert patched != before
            again()
            wait_for_js(pg, "/format 4/.test(document.body.innerText)", timeout=60000)
            assert pg.evaluate(HASH) == patched, "the index of the newer program was not touched"
            assert pg.evaluate("window.MZLAB_LR_MISS || []") == [], "no route unavailable"
        step("a newer index: warning, index untouched", newer)
        def known():
            pg.evaluate(VERSION, 3); again()
            wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.some(f => !f.gone)", timeout=60000)
            assert "format 4" not in pg.evaluate("document.body.innerText"), "no warning for a known version"
            assert pg.evaluate(HEADER) == 3
        step("a known version: no warning", known)
        def older():
            for old in (1, 2):
                # the page of the step before still has the index open: a fresh load lets go of it before the header is patched
                pg.goto(pg.url); wait_for_js(pg, "document.querySelector('#pick')")
                pg.evaluate(VERSION, old); again()
                wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.some(f => !f.gone)", timeout=60000)
                assert not pg.evaluate("/format \\d/.test(document.body.innerText)"), f"no warning for version {old}"
                assert pg.evaluate(HEADER) == 3, f"version {old} was rebuilt as the current one"
                assert pg.evaluate("window.MZLAB_LR_MISS || []") == [], f"no route unavailable after version {old}"
        step("an older index is rebuilt without a warning", older)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for nme, st in steps: print(("OK   " if st == "ok" else "FAIL ") + nme + ("" if st == "ok" else "  " + st))
r.report()
sys.exit(0 if all(s == "ok" for _, s in steps) else 1)
