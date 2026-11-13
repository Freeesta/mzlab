# e2e_annulla_orfani: «Cancel» of a loading while builders are indexing leaves in OPFS only the files of the table: the copy of a cancelled file and its `.part`
# index go even though the stopped builder's worker still holds them for a moment (archivio.js removeHeld). Synthetic files (tools/mzml_msconvert_synthetic.py),
# no laboratory data. Needs the site and the .wasm: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_annulla_orfani.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_synthetic as gen
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:400]))
# «Cancel» clicked as soon as one file is open and a builder is indexing another one (its row reads the scans)
ARM = """() => { window.__at = null; const t = setInterval(() => { const b = document.querySelector('[data-up-all]'), open = typeof E !== 'undefined' && E.files ? E.files.filter(f => !f.gone).length : 0;
  if (b && open >= 1 && CONVROWS.running().some(r => r.state === 'nscan' || r.state === 'idx')) { window.__at = CONVROWS.rows.map(r => r.name + ':' + r.state); b.click(); clearInterval(t); } }, 20); }"""
OPFS = """async () => { const root = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), o = { rows: ST.files.map(f => f.name) };
  for (const k of ['files', 'idx']) { o[k] = []; try { for await (const [n] of (await root.getDirectoryHandle(k)).entries()) o[k].push(n); } catch (_) { /* no folder */ } }
  return o; }"""
# true once OPFS holds only the files of the table and no `.part` (the same rule as leftovers() below)
CLEAN = "(" + OPFS + """)().then(o => { const rows = new Set(o.rows);
  return o.files.every(f => rows.has(f)) && o.idx.every(x => !x.endsWith('.part') && rows.has(x.replace('.cen.mzidx', '').replace('.mzidx', ''))); })"""
d = tempfile.mkdtemp(prefix="annulla_"); fs = []
for i in range(6):
    p = os.path.join(d, f"lento_{i}.mzML"); gen.scrivi(p, scans=6000, peaks1=1500, peaks2=150); fs.append(p)
def leftovers(o):
    rows = set(o["rows"])
    return sorted([f for f in o["files"] if f not in rows] + [x for x in o["idx"] if x.endswith(".part") or x.replace(".cen.mzidx", "").replace(".mzidx", "") not in rows])
def cancel_cleans():
    r = Run(port=8857, wd="/tmp/wd_annulla_orfani")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(ARM)
            pg.set_input_files("#pick", fs)
            wait_for_js(pg, "window.__at !== null", 300000)
            at = pg.evaluate("window.__at")
            wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0", 120000)
            try: wait_for_js(pg, CLEAN, 12000)      # the cancelled files go within a few seconds
            except TimeoutError: pass
            o = pg.evaluate(OPFS)
            assert o["rows"] and len(o["rows"]) < len(fs), f"the cancel came too late or too early: {at} → {o['rows']}"
            assert not leftovers(o), f"left in OPFS with no row after «Cancel»: {leftovers(o)} (rows at the cancel {at})"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("«Cancel» while builders index leaves in OPFS only the files of the table", cancel_cleans)
shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
