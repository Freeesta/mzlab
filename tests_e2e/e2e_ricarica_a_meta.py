# e2e_ricarica_a_meta: a file removed while others are still opening stays removed (explore.js bootSession0); after a reload of the page in the middle of a loading, the files
# still being read open again by themselves (no notebook names them yet) and the removed file stays unticked: a row is ticked exactly when its file is open (shell.js restoreRows).
# Synthetic files (tools/mzml_msconvert_sintetico.py), no laboratory data. Needs the site and the .wasm: python3 tools/prepara_sito.py, then python3 tests_e2e/e2e_ricarica_a_meta.py
import os, sys, tempfile, shutil; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
sys.path.insert(0, str(ROOT / "tools"))
import mzml_msconvert_sintetico as gen
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:600]))
d = tempfile.mkdtemp(prefix="ricarica_"); fs = []
for i in range(6):      # three small files open while the three large ones dropped after them are still being read
    p = os.path.join(d, f"lento_{i}.mzML"); gen.scrivi(p, scans=1500 if i < 3 else 12000, peaks1=1500, peaks2=150, seed=i + 1); fs.append(p)
# as soon as two files are open (the session keeps one after the removal) while others are still loading, one of them already in the table: the student removes an open file and, once the
# notebook holds that, the page reloads (sessionStorage carries what it saw)
ARM = """() => { const t = setInterval(async () => { const open = typeof ST !== 'undefined' && typeof OPENNOW !== 'undefined' ? ST.files.filter(x => OPENNOW.isOpen(x.name)) : [];
  if (open.length < 2 || UPBUSY === 0 || open.length === ST.files.length) return;      // a row of the table is still being opened
  clearInterval(t); const f = open[0]; OPENNOW.tick(f, false); uiSave(true);
  const saved = async () => { const nb = await (await fetch('api/notebook')).json(); return !(nb.session || []).some(x => String(x.file).split('#')[0] === f.name) && ((nb.ui || {}).files || []).some(x => x.gone && String(x.file).split('#')[0] === f.name); };
  while (!(await saved())) await new Promise(r => requestAnimationFrame(r));
  sessionStorage.setItem('ricarica', JSON.stringify([f.name, ST.files.filter(x => !OPENNOW.isOpen(x.name) && x.name !== f.name).map(x => x.name)])); location.reload(); }, 20); }"""
# the page after the reload, with its table
RELOADED = "performance.getEntriesByType('navigation')[0].type === 'reload' && sessionStorage.getItem('ricarica') !== null && typeof ST !== 'undefined' && ST.files.length >= 2"
# the rows of the table: ticked, open
ROWS = "typeof ST === 'undefined' || typeof OPENNOW === 'undefined' ? [] : ST.files.map(f => [f.name, !!f.use, OPENNOW.isOpen(f.name)])"
def reload_resumes():
    r = Run(port=8859, wd="/tmp/wd_ricarica_a_meta")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(ARM)
            pg.set_input_files("#pick", fs)
            attendi(pg, RELOADED, 300000)
            gone, late = pg.evaluate("JSON.parse(sessionStorage.getItem('ricarica'))")
            ok = f"(() => {{ const rows = {ROWS}; return rows.length >= 2 && rows.every(([n, use, open]) => n === {gone!r} ? !use && !open : use && open); }})()"
            try: attendi(pg, ok, 180000)
            except TimeoutError: raise AssertionError(f"after the reload (removed {gone}, still loading {late}) the rows [name, ticked, open] are {pg.evaluate(ROWS)}")
            assert any(n in late for n, _, _ in pg.evaluate(ROWS)), f"no file that was still loading came back: {late}"
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
# the same removal with no reload: the files opened after it do not bring the removed one back (explore.js bootSession0 keeps a removed entry)
RM = """() => { window.__rm = null; const t = setInterval(() => { const open = ST.files.filter(x => OPENNOW.isOpen(x.name));
  if (open.length < 2 || UPBUSY === 0) return; clearInterval(t); OPENNOW.tick(open[0], false); window.__rm = open[0].name; }, 20); }"""
def removal_holds():
    r = Run(port=8862, wd="/tmp/wd_ricarica_a_meta_rm")
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            pg.wait_for_selector("#start:not([hidden])", timeout=60000)
            pg.evaluate(RM)
            pg.set_input_files("#pick", fs)
            attendi(pg, "window.__rm !== null", 300000)
            gone = pg.evaluate("window.__rm")
            attendi(pg, f"ST.files.filter(f => f.name !== {gone!r}).length === {len(fs) - 1} && UPBUSY === 0 && OPENNOW.busy() === 0", 300000)      # the removed row goes when its notice closes
            rows = pg.evaluate(ROWS)
            assert not any(n == gone and op for n, _, op in rows), f"{gone}, removed while the others opened, is open again: {rows}"
            assert len([n for n, use, op in rows if use and op]) == len(fs) - 1, rows
            assert not [e for e in r.errs if e[0] == "pageerror"], r.errs
    finally:
        r.close()
step("a file removed while the others open stays removed", removal_holds)
step("reload mid-load: the files still loading open again, the removed one stays unticked", reload_resumes)
shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
