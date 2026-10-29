"""The last change of the notebook survives a reload that comes before the debounce (800 + 500 ms) has fired: with a file open (uiSave path) and on the
start screen (no file: only nbSave has something to flush). Synthetic file, invented names."""
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_taccuino_flush.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
tmp = Path(tempfile.mkdtemp(prefix="tf_"))
mzf = tmp / "X_t000min.mzML"; synth.write_full(mzf, 0, blank=False, seed=1)
r = Run(port=8830 + 160, wd="/tmp/wd_taccuino_flush")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        pg.wait_for_selector("#start:not([hidden])", timeout=60000)
        pg.wait_for_function("typeof nbEnsure === 'function'", timeout=60000)
        def quick_reload(mark):
            # the change and the reload come within ~50 ms: far inside the debounce
            pg.evaluate("m => { NB.flushMark = m; nbSave(); }", mark)
            hold(pg, 50)
            pg.reload(); pg.wait_for_function("typeof nbEnsure === 'function'", timeout=90000)
            pg.evaluate("nbEnsure()")
            return pg.evaluate("NB.flushMark")
        def start_screen():
            pg.evaluate("nbEnsure()")
            assert quick_reload("start-screen") == "start-screen"
        step("a change on the start screen (no file) survives an immediate reload", start_screen)
        def with_file():
            stage(pg, [mzf]); pg.click("#opbtn"); ready(pg)
            assert quick_reload("with-file") == "with-file"
        step("a change with a file open survives an immediate reload", with_file)
        def ui_change():
            pg.wait_for_function("typeof E !== 'undefined' && E.files.length === 1", timeout=90000); ready(pg)
            pg.evaluate("() => { NB.flushMark = 'ui'; uiSave(); }")
            hold(pg, 50)
            pg.reload(); pg.wait_for_function("typeof E !== 'undefined' && E.files.length === 1", timeout=90000)
            assert pg.evaluate("NB.flushMark") == "ui", "the debounced uiSave was lost"
        step("a change queued in uiSave (800 ms) survives an immediate reload", ui_change)
        pg.close()
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for n, s in steps: print(("PASS " if s == "ok" else "FAIL ") + n + ("" if s == "ok" else " -> " + s))
if bad: r.report(); sys.exit(1)
print("OK")
