# e2e_agilent_riaggiunto: an Agilent .D folder given again (the same files: names, sizes, dates) keeps its one sample (agilent-d.js toMzml, nativo.js cached): no second row, no second
# copy or index in OPFS, both in the same page and after a reload of the page (the session restored). Synthetic .D of tools/agilent_sintetico.py (invented numbers).
# Needs the site: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_agilent_riaggiunto.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import agilent_sintetico
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
D = Path(tempfile.mkdtemp(prefix="agri_"))
TOP = D / "serie"
agilent_sintetico.write(TOP, "uno.D", 120)
# every file of OPFS mzlab/ with its size
WALK = """async () => { const out = {}; const walk = async (h, p) => { for await (const [n, c] of h.entries()) { if (c.kind === 'file') out[p + n] = (await c.getFile()).size; else await walk(c, p + n + '/'); } };
  await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); return out; }"""
# upload() counted: a loading has ended when the count grows (the folder picker calls the global upload)
COUNT = "(() => { if (window.__ups === undefined) { const u = window.upload; window.__ups = 0; window.upload = async (...a) => { try { return await u(...a); } finally { window.__ups++; } }; } return window.__ups; })()"
IDLE = "UPBUSY === 0 && OPENNOW.busy() === 0"
def state(pg):
    o = pg.evaluate(f"({WALK})()")
    return {"rows": pg.evaluate("ST.files.map(f => f.name).sort()"),
            "copies": sorted(k for k in o if k.startswith("files/nat~")), "idx": sorted(k for k in o if k.startswith("idx/"))}
def give(pg):
    n = pg.evaluate(COUNT)
    pg.set_input_files("#pickdir", str(TOP))
    attendi(pg, f"window.__ups > {n} && {IDLE}", 180000)
def again():
    r = Run(port=8868, wd="/tmp/wd_agilent_riaggiunto")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            give(pg)
            attendi(pg, f"ST.files.length === 1 && OPENNOW.isOpen(ST.files[0].name) && {IDLE}", 180000)
            first = state(pg)
            assert first["rows"] == ["uno.mzML"] and len(first["copies"]) >= 1 and len(first["idx"]) == 1, first
            give(pg)
            same = state(pg)
            assert same == first, ("same page", first, same)
            assert pg.evaluate("CONVROWS.rows.some(r => r.name === 'uno.mzML' && r.state === 'done')"), "no «from the cache» row"
            pg.reload()
            attendi(pg, f"typeof ST !== 'undefined' && ST.files.length === 1 && OPENNOW.isOpen(ST.files[0].name) && {IDLE}", 180000)
            give(pg)
            after = state(pg)
            assert after == first, ("after a reload", first, after)
            assert pg.evaluate("ST.files.every(f => OPENNOW.isOpen(f.name))"), "the sample is not open"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("a .D given again keeps one row, one copy and one index (same page, then after a reload)", again)
shutil.rmtree(D, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
