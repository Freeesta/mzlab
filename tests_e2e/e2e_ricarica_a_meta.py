# e2e_ricarica_a_meta: a reload of the page while files are still loading. The files that were still being read open again by themselves (they were in no notebook
# yet) and a file the student removed before the reload stays unticked: every row of the table is ticked exactly when its file is open (shell.js restoreRows).
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
# as soon as two files are open (the session keeps one after the removal) while others are still loading: the student removes an open file and, once the
# notebook holds that, the page reloads (sessionStorage carries what it saw)
ARM = """() => { const t = setInterval(async () => { const open = typeof ST !== 'undefined' && typeof OPENNOW !== 'undefined' ? ST.files.filter(x => OPENNOW.isOpen(x.name)) : [];
  if (open.length < 2 || UPBUSY === 0) return;
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
step("reload mid-load: the files still loading open again, the removed one stays unticked", reload_resumes)
shutil.rmtree(d, ignore_errors=True)
print("\nSTEPS"); [print(" ", s) for s in steps]
if any(s[1] != "ok" for s in steps): sys.exit(1)
