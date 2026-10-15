"""A true reset: what a student loads stays in the browser after a reload (the page says so once); «Full reset» removes files, session, notebook, OPFS, IndexedDB,
caches, service workers and preferences by NAME (also folders and databases that this test invents, as other parts of the program do); «Files and session only» keeps the
preferences, the language and the saved libraries. The one entry is the header button «New session» (dialog with both and Cancel), also ?reset=all|1|files."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
EX = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_reset.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
r = Run(port=8889, wd="/tmp/wd_reset")
SEED = """async () => {
  const root = await navigator.storage.getDirectory();
  for (const n of ['mzlab', 'invented-by-another-part', 'librerie']) { const d = await root.getDirectoryHandle(n, { create: true }); const f = await d.getFileHandle('x.bin', { create: true }); const w = await f.createWritable(); await w.write(new Uint8Array([1, 2, 3])); await w.close(); }
  for (const n of ['mzlab-libs', 'mzlab-libdir', 'invented-db']) await new Promise(res => { const q = indexedDB.open(n, 1); q.onupgradeneeded = () => q.result.createObjectStore('s'); q.onsuccess = () => { q.result.close(); res(); }; q.onerror = res; });
  for (const n of ['qqq-big-x', 'qqq-app-x', 'other-cache']) await (await caches.open(n)).put('/a', new Response('1'));
  localStorage.setItem('mzlab.lang', 'en'); localStorage.setItem('qqq.mappa', '{"pal":"cividis"}'); localStorage.setItem('mzlab.invented', '1');
}"""
STATE = """async () => ({
  opfs: await (async () => { const out = []; for await (const [n] of (await navigator.storage.getDirectory()).entries()) out.push(n); return out.sort(); })(),
  mzlab: await (async () => { const out = []; const walk = async (d, pre) => { for await (const [n, h] of d.entries()) { if (h.kind === 'file') out.push(pre + n); else await walk(h, pre + n + '/'); } }; try { await walk(await (await navigator.storage.getDirectory()).getDirectoryHandle('mzlab'), ''); } catch (e) {} return out; })(),   // files left under mzlab/: the engine may create its folders again, empty, when the page starts
  idb: (await indexedDB.databases()).map(d => d.name).sort(),
  caches: (await caches.keys()).sort(),
  ls: Object.keys(localStorage).filter(k => /^(qqq|mzlab)/i.test(k)).sort(),
  files: typeof E !== 'undefined' ? E.files.length : -1, sess: typeof S !== 'undefined' && !!S.sess, url: location.search
})"""
try:
    with sync_playwright() as p:
        b = p.chromium.launch(); r.b = b
        ctx = b.new_context(locale="en-US", viewport={"width": 1280, "height": 900})
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        base = f"http://127.0.0.1:{r.port}/"
        def load_files():
            pg.goto(base); pg.wait_for_timeout(800)
            pg.evaluate("document.querySelectorAll('#tour-invito button').forEach(b => b.textContent.trim() === 'Not now' && b.click())")
            if not pg.evaluate("typeof E !== 'undefined' && E.files.length >= 2"):          # a page that resumed its session already has them
                stage(pg, [EX / "FullScan_t0.mzML", EX / "FullScan_t30.mzML"])
                pg.click("#opbtn"); ready(pg)
            assert pg.evaluate("E.files.length") >= 2
            pg.evaluate(SEED)
        def stay():
            load_files()
            pg.evaluate("localStorage.removeItem('qqq.reset.note')"); saved(pg); pg.reload(); ready(pg)
            attendi(pg, "E.files.length >= 2", 60000)
            assert pg.evaluate("E.files.length") >= 2, "after a reload the files are still there (this is wanted)"
            assert pg.locator("#resetnote").count() == 1 and "stay in the browser" in pg.inner_text("#resetnote"), "the page says so"
            pg.reload(); ready(pg); pg.wait_for_timeout(500); assert pg.locator("#resetnote").count() == 0, "only once"
        step("a reload keeps the files; the page says it once", stay)
        def partial():
            pg.goto(base + "?reset=files"); pg.wait_for_function("location.search === '' || !location.search.includes('reset')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE)
            assert "?reset" not in s["url"] and s["files"] == 0 and not s["sess"], s
            assert s["mzlab"] == [] and "invented-by-another-part" not in s["opfs"] and "librerie" in s["opfs"], s["opfs"]
            assert "invented-db" not in s["idb"] and "mzlab-libs" in s["idb"] and "mzlab-libdir" in s["idb"], s["idb"]
            assert "qqq-big-x" not in s["caches"] and "other-cache" in s["caches"], s["caches"]
            assert "mzlab.lang" in s["ls"] and "qqq.mappa" in s["ls"] and "mzlab.invented" in s["ls"], s["ls"]
        def ready_start(q):
            q.wait_for_function("typeof S !== 'undefined' && window.qqReady", timeout=60000); q.wait_for_timeout(500)
        step("?reset=files: no files or session, folders and databases removed by name, libraries and preferences stay", partial)
        def full_cmd():
            load_files(); pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]")
            txt = pg.inner_text("#resetdlg"); assert "Start a new session?" in txt and "Choose what to delete." not in txt and "Full reset" in txt and "Everything, as the first time" in txt and "Files and session only" in txt and "Cancel" in txt, txt
            hs = pg.evaluate("[...document.querySelectorAll('#resetdlg button')].map(b => b.getBoundingClientRect().height)"); assert all(h >= 44 for h in hs), hs
            assert pg.evaluate("(() => { const p = document.createElement('span'); p.style.color = 'var(--on-accent)'; p.style.background = 'var(--bad)'; document.body.appendChild(p); const c = getComputedStyle(p).backgroundColor; p.remove(); return c === getComputedStyle(document.querySelector('#resetdlg [data-reset=all]')).backgroundColor; })()"), "the wide button is red (CSS variable)"
            pg.click("#resetdlg [data-reset=all]"); pg.wait_for_function("location.search.includes('riparti')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE)
            assert s["files"] == 0 and not s["sess"], s
            assert [n for n in s["opfs"] if n != "mzlab"] == [] and s["mzlab"] == [], (s["opfs"], s["mzlab"])       # the engine may create its folder again, empty, at start
            assert set(s["idb"]) <= {"qqq_lab"}, s["idb"]
            assert s["caches"] == [] or all(c.startswith("qqq-app") for c in s["caches"]) and "other-cache" not in s["caches"], s["caches"]
            assert "mzlab.lang" not in s["ls"] and "mzlab.invented" not in s["ls"] and "qqq.mappa" not in s["ls"], s["ls"]
        step("Header «New session» → Full reset: dialog, then nothing left (OPFS, IndexedDB, caches, preferences)", full_cmd)
        def gear():
            load_files(); pg.click("#np-set"); pg.wait_for_timeout(400)
            txt = pg.inner_text("#uipset"); assert "Full reset" not in txt and "Reset completo" not in txt and "Storage in the browser" not in txt, txt
            assert pg.locator("#uip-arch-all").count() == 0, "no reset in the gear"
            pg.keyboard.press("Escape"); pg.keyboard.press("Control+k"); pg.keyboard.type("reset"); pg.wait_for_timeout(300)
            assert "Full reset" not in pg.inner_text("#pal-l"), "no reset in the palette"; pg.keyboard.press("Escape")
        step("gear and palette have no reset (the header button is the only entry)", gear)
        def url_all():
            load_files(); pg.goto(base + "?reset=1"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)
            s = pg.evaluate(STATE); assert s["files"] == 0 and [n for n in s["opfs"] if n != "mzlab"] == [] and s["mzlab"] == [] and "mzlab.lang" not in s["ls"], s
        step("?reset=1 is the full reset at start", url_all)
        def cancel():
            load_files(); pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]"); pg.click("#resetdlg .rs-cancel"); pg.wait_for_timeout(300)
            s = pg.evaluate(STATE); assert s["files"] >= 2 and not pg.locator("#resetdlg").count(), s
            pg.click("#newrun"); pg.wait_for_selector("#resetdlg[open]"); pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
            s = pg.evaluate(STATE); assert s["files"] >= 2 and not pg.locator("#resetdlg").count(), "Esc = Cancel"
        step("Cancel changes nothing", cancel)
        def remove_one():
            load_files(); pg.evaluate("redrawAll()"); pg.wait_for_timeout(1500)
            before = pg.evaluate(STATE)["mzlab"]; k = pg.evaluate("E.files[0].file.split('#')[0]"); assert any(k in n for n in before), (k, before)
            pg.evaluate("void removeFile(E.files[0])"); pg.wait_for_selector("#askdlg[open]"); pg.click("#askok")
            pg.wait_for_timeout(1500)
            after = pg.evaluate(STATE)["mzlab"]; assert not any(k in n for n in after), ("the file and its index are freed", k, after)
            assert pg.evaluate("E.files.filter(f => !f.gone).length") >= 1
        step("«Remove from the session» also frees the file and its indexes in OPFS", remove_one)
        def clear_all():
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)
            load_files(); pg.evaluate("STATO.aggiunta(true, 'adding'); applyView()"); pg.wait_for_selector("#clearall")
            assert pg.inner_text("#clearall") == "Delete all files" and pg.inner_text("#loaded").startswith("Files already loaded ("), pg.inner_text("#loaded")
            pg.click("#clearall"); assert "Really delete" in pg.inner_text("#clearall") and pg.evaluate("E.files.length") >= 2, "the first press only asks"
            pg.click("#clearall"); pg.wait_for_function("location.search === '' && window.qqReady && typeof E !== 'undefined' && E.files.length === 0", timeout=60000); pg.wait_for_timeout(800)
            s = pg.evaluate(STATE)
            assert s["files"] == 0 and not s["sess"] and s["mzlab"] == [], s                     # memory released: the files and their indexes are gone
            assert "librerie" in s["opfs"] and "mzlab.lang" in s["ls"], s                         # the same as «Files and session only»
            stage(pg, [EX / "FullScan_t0.mzML"]); pg.click("#opbtn"); ready(pg); assert pg.evaluate("E.files.length") >= 1, "loading again works"
        step("«Delete all files» of the card of the loaded files: two presses, empty list, memory released, loading again works", clear_all)
        def stuck():
            pg.goto(base + "?reset=files"); pg.wait_for_function("!location.search.includes('reset')", timeout=30000); ready_start(pg)
            assert pg.locator("#stuck").count() == 0, "not at once"
            pg.wait_for_selector("#stuckbtn", timeout=20000)
            assert "Full reset" in pg.inner_text("#stuckbtn") and "Not starting" in pg.inner_text("#stuck"), pg.inner_text("#stuck")
            pg.click("#stuckbtn"); pg.wait_for_selector("#resetdlg[open]")
            assert "Files and session only" in pg.inner_text("#resetdlg"), "the same dialog as «New session»"
            pg.keyboard.press("Escape"); pg.wait_for_selector("#resetdlg", state="detached")
        step("after 10 s on the start screen without files a «Full reset» button opens the same dialog", stuck)
        b.close()
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
