"""A file taken out from the list while the tab on screen is another one (synthetic pairs of tools/wiff_misto.py and tools/wiff_modi.py EPI: Full Scan and MS2
files; no laboratory data): the list shows the files of every tab, so Ctrl/Cmd+Z and the Undo button take that file back at once, from the tab on screen,
and Ctrl+Y takes it out again (storia.js: a removed file is an entry of no tab).
Needs the site and the .wasm (python3 tools/prepara_sito.py). Run from the repo root: python3 tests_e2e/e2e_annulla_file_scheda.py"""
import subprocess, sys, os, tempfile
from pathlib import Path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *

steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e)[:600]))

D = Path(tempfile.mkdtemp(prefix="annulla_scheda_"))
for tool in ("wiff_misto.py", "wiff_modi.py"):
    subprocess.run([sys.executable, str(ROOT / "tools" / tool), str(D)], check=True, capture_output=True)
FILES = [D / "misto.wiff", D / "misto.wiff.scan", D / "modi" / "EPI.wiff", D / "modi" / "EPI.wiff.scan"]
OPEN = "(() => { E._grpOpen = E._grpOpen || {}; E.files.forEach(f => { E._grpOpen[f.kind] = true; }); renderFileList(); })()"

r = Run(port=free_port(), wd="/tmp/wd_annulla_scheda")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        stage(pg, FILES); open_first(pg); ready(pg); drawer(pg)
        k = pg.evaluate("(E.files.find(f => !f.gone && f.kind !== E.tab) || {}).k")
        def remove():
            assert k is not None and pg.evaluate("E.tab") == "full", pg.evaluate("[E.tab, E.files.map(f => f.kind)]")
            pg.evaluate(OPEN); pg.locator(f"#flst .fl[data-k='{k}'] .fl-x").click(); attendi(pg, f"E.files[{k}].gone === true"); ready(pg)
            assert not pg.is_disabled("#np-undo")
        step("an MS2 file is taken out from the list while the Full Scan tab is on screen", remove)
        def undo_key():
            pg.mouse.click(5, 5); pg.keyboard.press("Control+z"); attendi(pg, f"E.files[{k}].gone === false"); ready(pg)
            assert pg.evaluate("E.tab") == "full"
        step("Ctrl+Z takes it back without leaving the tab", undo_key)
        def redo_key():
            pg.keyboard.press("Control+y"); attendi(pg, f"E.files[{k}].gone === true"); ready(pg)
        step("Ctrl+Y takes it out again", redo_key)
        def undo_button():
            pg.click("#np-undo"); attendi(pg, f"E.files[{k}].gone === false"); ready(pg)
            pg.evaluate(OPEN); assert pg.locator(f"#flst .fl[data-k='{k}']").count() == 1
        step("the Undo button takes it back and its row is in the list again", undo_button)
        step("no page error", lambda: (lambda e: e and (_ for _ in ()).throw(AssertionError(e)))([x for x in r.errs if x[0] == "pageerror"]))
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
    try: r.close()
    except Exception: pass
bad = [n for n, s in steps if s != "ok"]
for n, s in steps: print(s.split(" ")[0].upper(), n, s if s != "ok" else "")
r.report() if bad else None
sys.exit(1 if bad else 0)
