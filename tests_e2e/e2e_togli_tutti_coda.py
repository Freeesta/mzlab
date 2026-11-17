"""«Remove all files» frees every file of the list, also the ones read but not open yet: the copy in OPFS, its index and the row go, the queue of
OPENNOW does not open them afterwards and a reload brings none back; Undo within the window opens them all again. Synthetic files only.
The queue is held still by a slowed `post("api/explore", {append})`, as on a slow computer while a large drop is opening."""
# covers: mzlab/web/lista-file.js, mzlab/web/open-now.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import misura_lista_molti as mlm
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_togli_tutti_coda.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
N = 5
COUNT = ("async () => { const r = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'); const o = {}; for (const k of ['files', 'idx']) {"
         " let n = 0; try { for await (const _ of (await r.getDirectoryHandle(k)).keys()) n++; } catch (_) {} o[k] = n; } return o; }")
# the appends of the queue wait for window.__go: the first file opens, the second is in flight, the others wait in the queue
STALL = ("() => { window.__go = false; const p0 = window.__p0 || (window.__p0 = post); window.post = (u, b, raw) => u === 'api/explore' && b && b.append"
         " ? new Promise(ok => { const t = setInterval(() => { if (window.__go) { clearInterval(t); ok(p0(u, b, raw)); } }, 50); }) : p0(u, b, raw); }")
OPEN = "E.files.filter(f => !f.gone).length"
r = Run(port=8971, wd="/tmp/wd_togli_tutti_coda")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.wait_for_selector("#drop"); wait_for_js(pg, "window.qqReady === true", 60000)
        files = mlm.small_files(Path("/tmp/wd_togli_tutti_coda_in"), N)
        def drop_stalled():
            pg.evaluate(STALL); pg.set_input_files("#pick", files)
            wait_for_js(pg, f"UPBUSY === 0 && ST.files.length === {N} && {OPEN} === 1 && OPENNOW.busy() > 0", 120000)
        def remove_all_queued():
            drop_stalled()
            assert pg.evaluate(COUNT) == {"files": N, "idx": N}, pg.evaluate(COUNT)
            pg.evaluate("removeAllFiles()"); pg.evaluate("window.__go = true")
            wait_for_js(pg, "OPENNOW.busy() === 0", 60000)
            # the window of Undo (6 s) closes, then the copies are freed
            wait_for_js(pg, "ST.files.length === 0", 30000); assert pg.evaluate(OPEN) == 0, f"open after Remove all: {pg.evaluate(OPEN)}"
            pg.wait_for_function(COUNT.replace("return o; }", "return o.files === 0 && o.idx === 0; }"), timeout=30000)
            pg.reload(); wait_for_js(pg, "window.qqReady === true", 60000); hold(pg, 1500)
            assert pg.evaluate("ST.files.length") == 0 and pg.evaluate(OPEN) == 0, f"rows back after reload: {pg.evaluate('ST.files.length')}"
            assert pg.evaluate(COUNT) == {"files": 0, "idx": 0}, pg.evaluate(COUNT)
        step("Remove all while files wait to open: every copy, index and row is freed, none comes back after reload", remove_all_queued)
        def undo_queued():
            drop_stalled()
            pg.evaluate("removeAllFiles()"); wait_for_js(pg, f"{OPEN} === 0")
            assert pg.evaluate("STORIA.annulla()"), "nothing to undo"; pg.evaluate("window.__go = true")
            wait_for_js(pg, f"UPBUSY === 0 && OPENNOW.busy() === 0 && {OPEN} === {N}", 60000)
            hold(pg, 7000)      # past the window: an undone removal frees nothing
            assert pg.evaluate(OPEN) == N and pg.evaluate("ST.files.length") == N
            assert pg.evaluate("new Set(E.files.map(f => f.file)).size") == N, f"a file opened twice: {pg.evaluate('E.files.map(f => f.file)')}"
            assert pg.evaluate(COUNT) == {"files": N, "idx": N}, pg.evaluate(COUNT)
        step("Undo of Remove all while files wait to open: all of them open and keep their copies", undo_queued)
        def set_aside():
            pg.evaluate("window.__go = true; window.post = window.__p0; 0")
            name = os.path.basename(files[0]); pg.evaluate("removeAllFiles()"); wait_for_js(pg, "ST.files.length === 0", 30000)
            pg.wait_for_function(COUNT.replace("return o; }", "return o.files === 0 && o.idx === 0; }"), timeout=30000)
            pg.evaluate(f"OPENNOW.hold([{name!r}])"); pg.set_input_files("#pick", files)
            wait_for_js(pg, f"UPBUSY === 0 && OPENNOW.busy() === 0 && ST.files.length === {N} && {OPEN} === {N - 1}", 120000)
            pg.evaluate("removeAllFiles()"); wait_for_js(pg, "ST.files.length === 0", 30000)
            pg.wait_for_function(COUNT.replace("return o; }", "return o.files === 0 && o.idx === 0; }"), timeout=30000)
        step("Remove all also frees a file set aside (unticked, never opened)", set_aside)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
