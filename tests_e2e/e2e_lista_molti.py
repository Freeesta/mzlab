"""Data file list while many files load: the render is throttled during loading (explore.js renderFileList), and the list the student sees is the one the synchronous
renderer produces for the same state (renderFileList.now): after a load of 30 files, after closing a group and after removing a file. Synthetic files only."""
import sys, os, importlib.util; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
_s = importlib.util.spec_from_file_location("misura_lista_molti", str(Path(__file__).with_name("misura_lista_molti.py"))); _m = importlib.util.module_from_spec(_s); _s.loader.exec_module(_m)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:240]))
N = 30
SAME = "() => { const a = document.querySelector('#flst').innerHTML; renderFileList.now(); return a === document.querySelector('#flst').innerHTML; }"
r = Run(port=8903, wd="/tmp/wd_lista_molti_e2e")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        files = _m.small_files(Path("/tmp/wd_lista_molti_e2e/dati"), N)
        pg.evaluate("window.__renders = 0; const o = renderFileList.now; renderFileList.now = function () { window.__renders++; return o.apply(this, arguments); }")
        pg.set_input_files("#pick", files); loaded(pg); ready(pg)
        def load():
            assert pg.locator("#flst .fl[data-k]").count() == N, "rows"
            assert pg.evaluate("FLQ.tm") == 0, "no render left waiting"
            assert pg.evaluate("window.__renders") < N, "renders were throttled: " + str(pg.evaluate("window.__renders"))
        step("30 files: all rows listed, no render left waiting, fewer renders than files", load)
        step("identical to the synchronous render after the load", lambda: (_ for _ in ()).throw(AssertionError("list differs")) if not pg.evaluate(SAME) else None)
        def group():
            pg.click("#flst .fgh[data-gt] .chv"); ready(pg)
            assert pg.evaluate(SAME), "list differs after toggling a group"
        step("identical after toggling a group", group)
        def remove():
            pg.evaluate("document.querySelectorAll('#flst .fl-x')[3].click()"); ready(pg)
            assert pg.locator("#flst .fl[data-k]").count() == N - 1, "one row less"
            assert pg.evaluate(SAME), "list differs after removing a file"
        step("identical after removing a file", remove)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
