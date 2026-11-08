"""A file removed in another tab: tab B still lists it (the tabs are not kept in step), and ticking it says that it was removed in another tab or
window, with its name, instead of «This file cannot be read: {name}», which in its turn now names the file. Two tabs of one browser profile share the folder of the site. Synthetic files only."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import misura_lista_molti as mlm
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_tolto_altra_scheda.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
N = 2
INIT = ("try{sessionStorage.setItem('mzlab.coiReload','1');localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));"
        "localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}")
COPIES = ("async () => { const r = await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'); let n = 0;"
          " try { for await (const _ of (await r.getDirectoryHandle('files')).keys()) n++; } catch (_) {} return n; }")
r = Run(port=8972, wd="/tmp/wd_tolto_altra_scheda")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch(); ctx = r.b.new_context(viewport={"width": 1440, "height": 900}, reduced_motion="reduce")
        def tab():
            pg = ctx.new_page(); pg.add_init_script(INIT)
            pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
            pg.goto(f"http://127.0.0.1:{r.port}/"); wait_for_js(pg, "window.qqReady === true", 60000); return pg
        a = tab()
        files = mlm.small_files(Path("/tmp/wd_tolto_altra_scheda_in"), N)
        a.set_input_files("#pick", files); wait_for_js(a, f"UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.filter(f => !f.gone).length === {N}", 120000)
        b = tab(); wait_for_js(b, f"ST.files.length === {N} && E.files.filter(f => !f.gone).length === {N}", 60000)
        def removed_elsewhere():
            # B closes its open files and keeps the rows (the calls of «Switch to …», open-now-raro.js): its rows are read but not open
            b.evaluate("(async () => { closeAllPanels(); await post('api/new', {}); ST.files.forEach(f => { f.use = false; }); await bootSession(); renderFiles(); })()")
            wait_for_js(b, f"E.files.filter(f => !f.gone).length === 0 && ST.files.length === {N}", 30000)
            a.evaluate("removeAllFiles(); 0"); wait_for_js(a, "ST.files.length === 0", 30000)
            a.wait_for_function(COPIES.replace("return n; }", "return n === 0; }"), timeout=30000)
            name = os.path.basename(files[0])
            b.evaluate(f"OPENNOW.tick(ST.files.find(f => f.name === {name!r}), true); 0")
            wait_for_js(b, "OPENNOW.busy() === 0 && document.getElementById('err').textContent !== ''", 30000)
            msg = b.evaluate("document.getElementById('err').textContent")
            assert name in msg and "altra scheda" in msg, f"message: {msg!r}"
            assert b.evaluate(f"!ST.files.find(f => f.name === {name!r}).use"), "the row stays unticked"
            assert b.evaluate("E.files.filter(f => !f.gone).length") == 0
        step("ticking a row whose file another tab removed names the file and says where it went", removed_elsewhere)
        def index_lost():
            # the copy stays and only its index is gone: «This file cannot be read» with the name filled in
            g = mlm.small_files(Path("/tmp/wd_tolto_altra_scheda_in2"), 1)[0]; name = os.path.basename(g)
            a.set_input_files("#pick", [g]); wait_for_js(a, f"UPBUSY === 0 && OPENNOW.busy() === 0 && E.files.some(f => !f.gone && f.file === {name!r})", 120000)
            a.evaluate(f"(async () => {{ const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('idx'); await d.removeEntry({name + '.mzidx'!r}); }})()")
            msg = a.evaluate(f"post('api/explore', {{ samples: [{{ file: {name!r}, time: 0, type: 'sample', conc: null, cunit: null, label: null }}], append: true }}).then(() => '', e => String(e.message || e))")
            assert name in msg and "{name}" not in msg, f"message: {msg!r}"
        step("an index lost alone names the file in «This file cannot be read»", index_lost)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
