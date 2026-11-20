"""A true reset: what a student loads stays in the browser after a reload (the page says so once); «Full reset» removes files, session, notebook, OPFS, IndexedDB,
caches, service workers and preferences by NAME (also folders and databases that this test invents, as other parts of the program do); «Files» keeps the
preferences, the language and the saved libraries. The one entry is the header button «New session» (dialog with both and Cancel), also ?reset=all|1|files. Full reset also drops the program's own caches and worker (it is the guaranteed way to fetch the new version) and the page it reloads ends isolated by itself."""
# covers: mzlab/web/reset.js, mzlab/web/browser.js, mzlab/web/settings.js, mzlab/web/liste.js, mzlab/web/lr.js, mzlab/web/rust-worker.js, mzlab/web/qqq-db.js, mzlab/web/explore.js, mzlab/web/avvio-progressivo.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import time
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_reset.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " | ".join(l.strip() for l in str(e).split("\n") if l.strip())[:600]))
r = Run(port=8889, wd="/tmp/wd_reset")
SEED = """async () => {
  const root = await navigator.storage.getDirectory();
  for (const n of ['mzlab', 'invented-by-another-part', 'librerie']) { const d = await root.getDirectoryHandle(n, { create: true }); const f = await d.getFileHandle('x.bin', { create: true }); const w = await f.createWritable(); await w.write(new Uint8Array([1, 2, 3])); await w.close(); }
  for (const n of ['mzlab-libs', 'mzlab-libdir', 'invented-db']) await new Promise(res => { const q = indexedDB.open(n, 1); q.onupgradeneeded = () => q.result.createObjectStore('s'); q.onsuccess = () => { q.result.close(); res(); }; q.onerror = res; });
  for (const n of ['mzlab-big-x', 'mzlab-app-x', 'other-cache']) await (await caches.open(n)).put('/a', new Response('1'));
  localStorage.setItem('mzlab.map', '{"pal":"cividis"}'); localStorage.setItem('mzlab.invented', '1');
}"""
STATE = """async () => ({
  opfs: await (async () => { const out = []; for await (const [n] of (await navigator.storage.getDirectory()).entries()) out.push(n); return out.sort(); })(),
  mzlab: await (async () => { const out = []; const walk = async (d, pre) => { for await (const [n, h] of d.entries()) { if (h.kind === 'file') out.push(pre + n); else await walk(h, pre + n + '/'); } }; try { await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); } catch (e) {} return out; })(),   // files left under mzlab/: the engine may create its folders again, empty, when the page starts
  idb: (await indexedDB.databases()).map(d => d.name).sort(),
  caches: (await caches.keys()).sort(),
  ls: Object.keys(localStorage).filter(k => /^(qqq|mzlab)/i.test(k)).sort(),
  files: typeof E !== 'undefined' ? E.files.length : -1, sess: typeof S !== 'undefined' && !!S.sess, url: location.search
})"""
SEED_LISTS = """async () => {
  await LISTE.ready();                                   // the lists code opens its connection to mzlab_lab here
  const { DB_NAME, DB_VER, upgrade } = await import('/static/qqq-db.js');
  const db = await new Promise((ok, no) => { const q = indexedDB.open(DB_NAME, DB_VER); q.onupgradeneeded = () => upgrade(q.result); q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); });
  await new Promise((ok, no) => { const t = db.transaction(['user_lists', 'molecules'], 'readwrite'); t.objectStore('user_lists').put({ id: 'usr_reset', name: 'x', items: [] }); t.objectStore('molecules').put({ id: 'mol_reset' }); t.oncomplete = ok; t.onerror = () => no(t.error); });
  db.close();
}"""
COUNT_LISTS = """async () => {
  const { DB_NAME, DB_VER, upgrade } = await import('/static/qqq-db.js');
  const db = await new Promise((ok, no) => { const q = indexedDB.open(DB_NAME, DB_VER); q.onupgradeneeded = () => upgrade(q.result); q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); });
  const n = s => new Promise((ok, no) => { const q = db.transaction(s).objectStore(s).count(); q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); });
  const r = [await n('user_lists'), await n('molecules')]; db.close(); return r;
}"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1280, "height": 900})
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        base = f"http://127.0.0.1:{r.port}/"
        def load_files():
            for _try in range(3):          # the page left by a reset may still be doing its one isolating reload: that navigation aborts ours, the next one lands
                try: pg.goto(base); break
                except Exception as e:
                    if "ERR_ABORTED" not in str(e) or _try == 2: raise
                    wait_for_js(pg, "document.readyState === 'complete'", 10000)
            pg.wait_for_function("typeof S !== 'undefined' && window.qqReady", timeout=60000)
            if pg.evaluate("!!S.sess"): wait_for_js(pg, "E.files.length >= 2", 60000)          # a session left by the previous step resumes its files
            pg.evaluate("document.querySelectorAll('#tour-invito button').forEach(b => b.textContent.trim() === 'Not now' && b.click())")
            if not pg.evaluate("typeof E !== 'undefined' && E.files.length >= 2"):          # a page that resumed its session already has them
                stage(pg, [EX / "FullScan_t0.mzML", EX / "FullScan_t30.mzML"])
                pg.click("#opbtn"); ready(pg)
            assert pg.evaluate("E.files.length") >= 2
            pg.evaluate(SEED)
        def stay():
            load_files()
            pg.evaluate("localStorage.removeItem('mzlab.reset.note')"); saved(pg); pg.reload(); ready(pg)
            wait_for_js(pg, "E.files.length >= 2", 60000)
            assert pg.evaluate("E.files.length") >= 2, "after a reload the files are still there (this is wanted)"
            assert pg.locator("#resetnote").count() == 0, "no reminder pops up (switched off in reset.js)"
        step("a reload keeps the files and shows no reminder", stay)
        def partial():
            pg.goto(base + "?reset=files"); pg.wait_for_function("location.search === '' || !location.search.includes('reset')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE)
            assert "?reset" not in s["url"] and s["files"] == 0 and not s["sess"], s
            assert s["mzlab"] == [] and "invented-by-another-part" not in s["opfs"] and "librerie" in s["opfs"], s["opfs"]
            assert "invented-db" not in s["idb"] and "mzlab-libs" in s["idb"] and "mzlab-libdir" in s["idb"], s["idb"]
            assert "mzlab-big-x" not in s["caches"] and "other-cache" in s["caches"], s["caches"]
            assert "mzlab.map" in s["ls"] and "mzlab.invented" in s["ls"], s["ls"]
        def ready_start(q):
            q.wait_for_function("typeof S !== 'undefined' && window.qqReady", timeout=60000); q.wait_for_timeout(500)
        step("?reset=files: no files or session, folders and databases removed by name, libraries and preferences stay", partial)
        def full_cmd():
            load_files(); pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]")
            txt = pg.inner_text("#resetdlg"); assert "Start a new session?" in txt and "Choose what to delete." not in txt and "Full reset" in txt and "Everything, as the first time" not in txt and "Keeps preferences" not in txt and "Cancel" not in txt, txt
            lab = pg.evaluate("[...document.querySelectorAll('#resetdlg button')].map(b => [b.dataset.reset, b.textContent.trim()])")
            assert lab == [["files", "Files"], ["all", "Full reset"], ["", "×"]], ("the milder button first, the red one second, then the close X", lab)
            x = pg.locator("#resetdlg .rs-x"); assert x.get_attribute("aria-label") == "Close" and x.get_attribute("title") == "Close"
            bx = x.bounding_box(); assert bx["width"] >= 24 and bx["height"] >= 24, bx
            assert pg.evaluate("document.activeElement.classList.contains('rs-x')"), "the first focus is the close X, never the destructive button"
            hs = pg.evaluate("[...document.querySelectorAll('#resetdlg .rs-big')].map(b => b.getBoundingClientRect().height)"); assert all(round(h, 2) >= 44 for h in hs), hs
            assert pg.evaluate("(() => { const p = document.createElement('span'); p.style.color = 'var(--on-accent)'; p.style.background = 'var(--bad)'; document.body.appendChild(p); const c = getComputedStyle(p).backgroundColor; p.remove(); return c === getComputedStyle(document.querySelector('#resetdlg [data-reset=all]')).backgroundColor; })()"), "the wide button is red (CSS variable)"
            pg.click("#resetdlg [data-reset=all]"); pg.wait_for_function("location.search.includes('riparti')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE)
            assert s["files"] == 0 and not s["sess"], s
            assert [n for n in s["opfs"] if n != "mzlab"] == [] and s["mzlab"] == [], (s["opfs"], s["mzlab"])       # the engine may create its folder again, empty, at start
            assert set(s["idb"]) <= {"mzlab_lab", "qqq_lab"}, s["idb"]
            # the fresh page may fill its own caches again (the app, and the idle prefetch of OpenChemLib in the big cache): the seeded ones are gone
            assert all(c.startswith("mzlab-app") or c.startswith("mzlab-big-") and c != "mzlab-big-x" for c in s["caches"]), s["caches"]
            assert "mzlab.invented" not in s["ls"] and "mzlab.map" not in s["ls"], s["ls"]
        step("Header «New session» → Full reset: dialog, then nothing left (OPFS, IndexedDB, caches, preferences)", full_cmd)
        def isolated_after():
            # a visitor's tab (no guard preset by the harness): first visit isolates itself, a Full reset must end isolated again with no manual reload
            c2 = b.new_context(locale="en-US", viewport={"width": 1280, "height": 900}); q = c2.new_page()
            q.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({state:'skipped'}));}catch(_){}")
            loads = []; q.on("framenavigated", lambda f: loads.append(f.url) if f == q.main_frame else None)
            try:
                q.goto(base); q.wait_for_function("crossOriginIsolated === true && window.qqReady", timeout=60000)
                def warm_ms():
                    t0 = time.time(); q.reload(); q.wait_for_function("window.qqReady", timeout=60000); return (time.time() - t0) * 1000
                warm = min(warm_ms(), warm_ms())
                n = len(loads); q.evaluate("void QQRESET.run('all')")      # not awaited: the reload ends the page's context
                q.wait_for_function("location.search.includes('riparti') || location.search.includes('ripn')", timeout=30000)
                q.wait_for_function("crossOriginIsolated === true && window.qqReady", timeout=60000)
                q.wait_for_timeout(1500)
                assert q.evaluate("crossOriginIsolated") and q.evaluate("!!navigator.serviceWorker.controller"), "isolated and controlled by the new worker after the reset"
                made = [u for u in loads[n:] if "ripn" in u]; assert len(made) == 2, ("the reset load plus ONE isolating reload (the engine then cleans the address, which is no load)", loads[n:])
                after = min(warm_ms(), warm_ms())
                assert after <= max(2 * warm, warm + 1500), ("a reload after a reset is as fast as a warm one", round(warm), round(after))
                print("   warm reload ms:", round(warm), "after reset ms:", round(after))
            finally: c2.close()
        step("after a Full reset the page is isolated with no manual reload, and the next reload is warm", isolated_after)
        def hard_cmd():
            # the start banner and the 10-s button call qqHardReset: the same wipe as the dialog's «Full reset», libraries included, a second call ignored
            load_files()
            pg.evaluate("window.qqHardReset(); window.qqHardReset()")
            pg.wait_for_function("location.search.includes('riparti')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE)
            assert s["files"] == 0 and [n for n in s["opfs"] if n != "mzlab"] == [] and s["mzlab"] == [], (s["opfs"], s["mzlab"])
            assert "librerie" not in s["opfs"] and not {"invented-db", "mzlab-libs", "mzlab-libdir"} & set(s["idb"]), (s["opfs"], s["idb"])
            assert "mzlab-big-x" not in s["caches"] and "mzlab-app-x" not in s["caches"] and "other-cache" not in s["caches"], s["caches"]
        step("qqHardReset (start banner, 10-s button) is the same Full reset: no folder, database or cache left, libraries too", hard_cmd)
        def gear():
            load_files(); pg.click("#np-set"); pg.wait_for_timeout(400)
            txt = pg.inner_text("#uipset"); assert "Full reset" not in txt and "Storage in the browser" not in txt, txt
            assert pg.locator("#uip-arch-all").count() == 0, "no reset in the gear"
            pg.keyboard.press("Escape"); pg.keyboard.press("Control+k"); pg.keyboard.type("reset"); pg.wait_for_timeout(300)
            assert "Full reset" not in pg.inner_text("#pal-l"), "no reset in the palette"; pg.keyboard.press("Escape")
        step("gear and palette have no reset (the header button is the only entry)", gear)
        def url_all():
            load_files(); pg.goto(base + "?reset=1"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE); assert s["files"] == 0 and [n for n in s["opfs"] if n != "mzlab"] == [] and s["mzlab"] == [], s
        step("?reset=1 is the full reset at start", url_all)
        def cancel():
            load_files(); pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]"); pg.click("#resetdlg .rs-x"); pg.wait_for_timeout(300)
            s = pg.evaluate(STATE); assert s["files"] >= 2 and not pg.locator("#resetdlg").count(), s
            assert pg.evaluate("document.activeElement && document.activeElement.id") == "newrun", "focus returns to the opener"
            pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]"); pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            s = pg.evaluate(STATE); assert s["files"] >= 2 and not pg.locator("#resetdlg").count(), "Esc = close"
        step("Cancel changes nothing", cancel)
        def remove_one():
            load_files(); pg.evaluate("redrawAll()"); pg.wait_for_timeout(1500)
            before = pg.evaluate(STATE)["mzlab"]; k = pg.evaluate("E.files[0].file.split('#')[0]"); assert any(k in n for n in before), (k, before)
            pg.evaluate("void removeFile(E.files[0])"); pg.wait_for_function("STORIA.pila.v.some(e => e.tipo === 'tolto')")
            assert any(k in n for n in pg.evaluate(STATE)["mzlab"]), "while the removal can be undone the copy stays"
            pg.wait_for_function("!STORIA.pila.v.some(e => e.tipo === 'tolto')", timeout=15000)      # the undo window closes without an undo: the copy is freed
            pg.wait_for_timeout(1500)
            after = pg.evaluate(STATE)["mzlab"]; assert not any(k in n for n in after), ("the file and its index are freed", k, after)
            assert pg.evaluate("E.files.filter(f => !f.gone).length") >= 1
        step("«Remove from the session» also frees the file and its indexes in OPFS", remove_one)
        def clear_all():
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)
            load_files()
            pg.evaluate("location.hash = '#/data'")                                               # a hash in the address must not stop the reload
            pg.evaluate("QQRESET.run('files')"); pg.wait_for_function("location.search === '' && window.qqReady && typeof E !== 'undefined' && E.files.length === 0", timeout=60000)
            wait_for_js(pg, "(async () => { const n = []; for await (const [k] of (await navigator.storage.getDirectory()).entries()) n.push(k); return n.includes('librerie') && !S.sess; })()", 30000)   # settled: the reset finished writing
            s = pg.evaluate(STATE)
            assert s["files"] == 0 and not s["sess"] and s["mzlab"] == [], s                     # memory released: the files and their indexes are gone
            assert "librerie" in s["opfs"] and "mzlab.map" in s["ls"], s                         # the same as «Files»
            stage(pg, [EX / "FullScan_t0.mzML"]); pg.click("#opbtn"); ready(pg); assert pg.evaluate("E.files.length") >= 1, "loading again works"
        step("«Files»: empty list, memory released, loading again works", clear_all)
        def keep_work():
            # what the user made survives «Files»: the drawing, a saved list, a saved molecule; the loaded files and the open-files list do not
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)      # a clean start: the step before leaves a session
            pg.evaluate("location.hash = '#/draw'"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            KJ = "document.querySelector('#kframe').contentWindow.ketcher"
            wait_for_js(pg, f"!!({KJ} && {KJ}.editor)"); pg.evaluate(f"{KJ}.setMolecule('CC(=O)Nc1ccc(O)cc1')"); wait_for_js(pg, f"{KJ}.editor.struct().atoms.size === 11")
            NBKET = "(async () => { try { return !!JSON.parse(await (await fetch('api/notebook')).text()).ket; } catch (e) { return false; } })()"
            wait_for_js(pg, NBKET, 15000)                                                       # the notebook holds the drawing
            pg.evaluate("""(async () => { const db = await new Promise((ok, no) => { const q = indexedDB.open(NOMI.DB); q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); });
              await new Promise((ok, no) => { const t = db.transaction(['user_lists', 'molecules'], 'readwrite'); t.objectStore('user_lists').put({ id: 'keep-list', name: 'my list' }); t.objectStore('molecules').put({ id: 'keep-mol', name: 'my molecule' }); t.oncomplete = ok; t.onerror = () => no(t.error); }); db.close(); })()""")
            load_files(); pg.evaluate("location.hash = '#/draw'")
            wait_for_js(pg, "(async () => { try { return (JSON.parse(await (await fetch('api/notebook')).text()).session || []).length > 0; } catch (e) { return false; } })()", 15000)   # the notebook names the open files
            # a change not saved yet when the button is pressed: the closing page must neither lose it nor write its old notebook (file list) back over the reset
            pg.evaluate("NB.keepProbe = 'unsaved'")
            # the dying page flushes once more right after the wipe (pagehide), as a real unload may: it must not bring the old notebook back
            pg.evaluate("(() => { const r = Storage.prototype.removeItem; Storage.prototype.removeItem = function (k) { if (k === NOMI.KEY('sessione')) { NB.late = 1; dispatchEvent(new Event('pagehide')); } return r.call(this, k); }; })()")
            pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]"); pg.click("#resetdlg [data-reset=files]")
            pg.wait_for_function("location.search === '' && window.qqReady && typeof E !== 'undefined' && E.files.length === 0", timeout=60000)
            got = pg.evaluate("""(async () => { const db = await new Promise(ok => { const q = indexedDB.open(NOMI.DB); q.onsuccess = () => ok(q.result); });
              const one = (st, k) => new Promise(ok => { const g = db.transaction(st).objectStore(st).get(k); g.onsuccess = () => ok(g.result || null); }), all = st => new Promise(ok => { const g = db.transaction(st).objectStore(st).count(); g.onsuccess = () => ok(g.result); });
              const r = { list: await one('user_lists', 'keep-list'), mol: await one('molecules', 'keep-mol'), files: await all('files'), manifest: await all('manifest') }; db.close(); return r; })()""")
            assert got["list"] and got["mol"], ("the saved list and molecule stay", got)
            assert got["files"] == 0 and got["manifest"] == 0, ("the loaded files are gone", got)
            wait_for_js(pg, NBKET, 30000)
            assert pg.evaluate("(async () => (JSON.parse(await (await fetch('api/notebook')).text()).session || []).length)()") == 0, "the open-files list is gone"
            assert pg.evaluate("(async () => JSON.parse(await (await fetch('api/notebook')).text()).keepProbe)()") == "unsaved", "the last change was saved before the reset"
            assert pg.evaluate("localStorage.getItem('mzlab.nbpending')") is None or "session" not in pg.evaluate("JSON.parse(localStorage.getItem('mzlab.nbpending') || '{}')"), "no pending copy names the files"
            pg.click("#nav [data-v=draw]"); pg.wait_for_function("!!(window.TPDraw && TPDraw.ready())", timeout=60000)
            wait_for_js(pg, f"!!({KJ} && {KJ}.editor) && {KJ}.editor.struct().atoms.size === 11", 30000)
        step("«Files» keeps the drawing, a saved list and a saved molecule, and removes the loaded files (the full reset removes all, see above)", keep_work)
        def lists_not_blocking():
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)      # no session left by the previous step
            load_files(); pg.evaluate(SEED_LISTS)
            rep = pg.evaluate("QQRESET.wipe('all')")
            assert "mzlab_lab" not in rep["blocked"] and "mzlab_lab" in rep["idb"], rep           # the page holds no idle connection to its own database
        step("Full reset in one tab: the user-lists code does not block deleting mzlab_lab", lists_not_blocking)
        def lists_gone():
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)      # no session left by the previous step
            load_files(); pg.evaluate(SEED_LISTS)
            assert pg.evaluate(COUNT_LISTS) == [1, 1]
            pg.evaluate("void QQRESET.run('all')"); pg.wait_for_function("location.search.includes('riparti') || location.search.includes('ripn')", timeout=30000)
            pg.wait_for_function("window.qqReady", timeout=60000)
            assert pg.evaluate(COUNT_LISTS) == [0, 0], "user lists and molecules do not survive a Full reset"
        step("Full reset empties user lists and molecules", lists_gone)
        # «Files» that the old page could not finish (an OPFS file still held, a database held by another connection): the new page's engine finishes it
        # (?riparti=file) and keeps what the dialog keeps; before, a held file came back, and a blocked database sent ?riparti=1, which also cleared the notebook
        HOLD = """async () => {                                   // a sync handle on a stored file, held by a worker of the page as an index build or a conversion does
          const src = `onmessage = async () => { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files');
            for (let k = 0; k < 100; k++) { for await (const [n, h] of d.entries()) { if (h.kind !== 'file') continue; try { self.h = await h.createSyncAccessHandle(); postMessage(n); return; } catch (e) { /* the engine holds it a moment */ } }
              await new Promise(r => setTimeout(r, 100)); } postMessage(''); };`;
          const w = new Worker(URL.createObjectURL(new Blob([src], { type: 'text/javascript' })));
          const name = await new Promise(r => { w.onmessage = e => r(e.data); w.postMessage(0); });
          // released right after the wipe gave up, before the reload: the wipe stores its report in QQRESET.last last of all
          Object.defineProperty(QQRESET, 'last', { configurable: true, get() { return this._last; }, set(v) { w.terminate(); this._last = v; } });
          QQRESET.opfsMs = 1500;
          return name;
        }"""
        WORK = """async (ket) => {                                  // the drawing as draw.js saves it, a saved list and a saved molecule in their stores
          NB.ket = ket; await nbSave(true);
          const db = await new Promise((ok, no) => { const q = indexedDB.open(NOMI.DB); q.onsuccess = () => ok(q.result); q.onerror = () => no(q.error); });
          await new Promise((ok, no) => { const t = db.transaction(['user_lists', 'molecules'], 'readwrite'); t.objectStore('user_lists').put({ id: 'held-list', name: 'my list' }); t.objectStore('molecules').put({ id: 'held-mol', name: 'my molecule' }); t.oncomplete = ok; t.onerror = () => no(t.error); }); db.close();
        }"""
        KEPT = """async () => { const db = await new Promise(ok => { const q = indexedDB.open(NOMI.DB); q.onsuccess = () => ok(q.result); });
          const one = (st, k) => new Promise(ok => { const g = db.transaction(st).objectStore(st).get(k); g.onsuccess = () => ok(g.result || null); });
          const r = { list: !!(await one('user_lists', 'held-list')), mol: !!(await one('molecules', 'held-mol')), ket: (JSON.parse(await (await fetch('api/notebook')).text() || '{}') || {}).ket || null }; db.close(); return r; }"""
        def held(mode, hold):
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)      # a clean start
            load_files(); ket = "CC(=O)Nc1ccc(O)cc1"; pg.evaluate(WORK, ket)
            q = None
            if hold == "opfs":
                assert pg.evaluate(HOLD), "a stored file is held"
            else:
                # a second page of the same browser keeps connections open and never closes them on versionchange: to the main database (which «Files» keeps
                # and so does not delete) and to one «Files» deletes, whose deletion therefore stays blocked
                q = ctx.new_page(); q.goto(base + "static/qqq-db.js")
                q.evaluate("""async () => { window.held = [];
                  for (const n of ['mzlab_lab', 'held-by-another-page']) window.held.push(await new Promise((ok, no) => { const r = indexedDB.open(n); r.onupgradeneeded = () => r.result.createObjectStore('s'); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); })); }""")
            urls = []; on = lambda f: urls.append(f.url) if f == pg.main_frame else None; pg.on("framenavigated", on)
            try:
                pg.evaluate(f"void QQRESET.run('{mode}')")
                pg.wait_for_function("window.qqReady && typeof E !== 'undefined' && E.files.length === 0 && !location.search.includes('riparti')", timeout=90000)
                wait_for_js(pg, "(async () => { try { const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab')).getDirectoryHandle('files'); for await (const _ of d.keys()) return false; } catch (e) {} return !S.sess; })()", 30000)      # settled
                s = pg.evaluate(STATE); k = pg.evaluate(KEPT)
            finally:
                pg.remove_listener("framenavigated", on)
                if q: q.close()
            back = [u for u in urls if "riparti=" in u]
            assert back and all(("riparti=file" if mode == "files" else "riparti=1") in u for u in back), ("the new page finishes the reset", urls)
            assert s["files"] == 0 and not s["sess"], s
            assert not [n for n in s["mzlab"] if n.startswith(("files/", "idx/"))], ("no stored file nor index comes back", s["mzlab"])
            return k
        def files_held_opfs():
            k = held("files", "opfs")
            assert k == {"list": True, "mol": True, "ket": "CC(=O)Nc1ccc(O)cc1"}, ("the drawing, the list and the molecule stay", k)
        step("«Files» with a stored file still held: the reload finishes it (?riparti=file), the drawing, a saved list and a molecule stay", files_held_opfs)
        def files_held_db():
            k = held("files", "db")
            assert k == {"list": True, "mol": True, "ket": "CC(=O)Nc1ccc(O)cc1"}, ("the drawing, the list and the molecule stay", k)
        step("«Files» with a database held by another page: the same, the notebook is not cleared", files_held_db)
        def all_held_opfs():
            k = held("all", "opfs")
            assert k == {"list": False, "mol": False, "ket": None}, ("the full reset still removes all", k)
        step("Full reset with a stored file still held: unchanged, ?riparti=1 and nothing of the user stays", all_held_opfs)
        def no_marker_after(mode):
            # the page that follows a reset must not start in «restoring»: pagehide of the old page may not write the marker back
            pg.add_init_script("try { window.__sessAtStart = localStorage.getItem('mzlab.session'); } catch (e) { window.__sessAtStart = 'err'; }")
            load_files()
            pg.evaluate(f"void QQRESET.run('{mode}')"); pg.wait_for_function("location.search.includes('riparti') || location.search.includes('ripn') || !location.search.includes('reset')", timeout=30000)
            # checked as soon as the new document ran its init script, before the engine is ready (afterwards the skeleton is gone anyway)
            # one condition evaluated in the page itself: the isolating reload may end this document between two separate calls
            pg.wait_for_function("'__sessAtStart' in window && window.__sessAtStart === null && !document.querySelector('#opening.restoring:not([hidden])') && !document.querySelector('#v-data.opn')", timeout=30000)
            pg.wait_for_function("window.qqReady && (crossOriginIsolated || " + ("false" if mode == "all" else "true") + ")", timeout=60000)      # after a Full reset the page reloads once to be isolated: the next step starts after it
        def pagehide_all(): no_marker_after("all")
        step("Full reset: the closing page does not write the session marker back", pagehide_all)
        def pagehide_files(): no_marker_after("files")
        step("Files reset: the session marker is gone at the next start", pagehide_files)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
