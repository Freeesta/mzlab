"""Warm-up of the pointed folder (riscalda.js, rust-scalda.js, docs/agenti/riscaldamento.md), synthetic files, dev server. The folder is a REAL FileSystemDirectoryHandle
(a folder of the browser's private file system, as in e2e_cartella_puntata). Steps: the mzML are copied and indexed in the background in list order and nothing shows (no row,
no session, still hidden after a reload); a click on a warm file loads it without copying or indexing (no «load» phase, the index bytes are the ones of the warm-up); the same
file loaded cold in another browser gives the same index bytes and the same TIC; a click on a file still waiting takes it out of the queue."""
import sys, os, base64, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="riscalda_e2e_")); NAMES = ["a.mzML", "b.mzML", "c.mzML"]
for i, n in enumerate(NAMES): synth.write_full(D / n, 5 + 5 * i, seed=1 + i)
B64 = {n: base64.b64encode((D / n).read_bytes()).decode() for n in NAMES}
INIT = """(() => { window.__ev = []; addEventListener('mzlab-index', e => window.__ev.push([e.detail.name, e.detail.phase || '']));
  window.showDirectoryPicker = async () => { const r = await navigator.storage.getDirectory(); return r.getDirectoryHandle('prova', { create: true }); };})()"""
PUT = """async ([path, b64]) => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('prova', { create: true });
  const w = await (await d.getFileHandle(path, { create: true })).createWritable(); await w.write(Uint8Array.from(atob(b64), c => c.charCodeAt(0))); await w.close(); }"""
SHA = """async (p) => { try { let d = await navigator.storage.getDirectory(); d = await d.getDirectoryHandle('mzlab'); const [k, n] = p.split('/'); d = await d.getDirectoryHandle(k);
  const f = await (await d.getFileHandle(n)).getFile(); const h = await crypto.subtle.digest('SHA-256', await f.arrayBuffer()); return Array.from(new Uint8Array(h)).map(b => b.toString(16).padStart(2, '0')).join(''); } catch (e) { return null; } }"""
NOSTART = "try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"
def finished(pg): return pg.evaluate("window.RISCALDA ? window.RISCALDA.state().finished : {}")
def shown(pg): return pg.evaluate("[ST.files.length, document.querySelectorAll('#flist input[data-k=use]').length, document.querySelectorAll('#fl-up .fl').length]")
r = Run(port=8830 + 160, wd="/tmp/wd_riscalda"); hot = {}
try:
    with sync_playwright() as p:
        kw = dict(viewport={"width": 1500, "height": 2200}, locale="it-IT", reduced_motion="reduce")
        try: r.b = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="riscalda_prof_"), **kw)
        except Exception:          # a Playwright without its own headless shell (the cloud image): the Chromium that is installed
            exe = next(Path("/opt/pw-browsers").glob("chromium-*/chrome-linux64/chrome"), None); assert exe, "no Chromium"
            r.b = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="riscalda_prof_"), executable_path=str(exe), **kw)
        pg = r.b.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script(NOSTART); pg.add_init_script(INIT)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#cartpick", timeout=60000)
        for n in NAMES[:2]: pg.evaluate(PUT, [n, B64[n]])
        def background():
            pg.click("#cartpick"); pg.wait_for_selector("#cartbody .cprow")
            pg.wait_for_function("window.RISCALDA && Object.keys(window.RISCALDA.state().finished).length >= 2", timeout=120000)
            f = finished(pg); assert list(f.items()) == [("a.mzML", "warmed"), ("b.mzML", "warmed")], f
            assert shown(pg) == [0, 0, 0], shown(pg)
            for n in NAMES[:2]:
                hot[n] = pg.evaluate(SHA, "idx/" + n + ".mzidx"); assert hot[n], n
                assert pg.evaluate(SHA, "files/" + n), n
                assert pg.evaluate(SHA, "warm/" + n + ".json"), n
        step("the files are prepared in list order and nothing shows", background)
        def hidden_after_reload():
            pg.reload(); pg.wait_for_selector("#cartbody .cprow", timeout=60000)
            pg.wait_for_function("typeof ST !== 'undefined' && window.MZLAB_LR", timeout=60000); pg.wait_for_timeout(1500)
            assert shown(pg) == [0, 0, 0], shown(pg)
        step("warm files stay out of the list after a reload", hidden_after_reload)
        def click_hit():
            pg.evaluate("window.__ev.length = 0")
            pg.click('.cprow:has-text("a.mzML")'); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=90000)
            pg.wait_for_function("document.querySelector('.cprow.done')")
            ev = pg.evaluate("window.__ev.filter(e => e[0] === 'a.mzML').map(e => e[1])"); assert "load" not in ev, ev
            assert pg.evaluate(SHA, "idx/a.mzML.mzidx") == hot["a.mzML"], "the index was built again"
            assert pg.evaluate(SHA, "warm/a.mzML.json") is None, "the marker stays after the click"
        step("a click on a warm file loads it with no copy and no index", click_hit)
        def waiting():
            pg.evaluate(PUT, ["c.mzML", B64["c.mzML"]]); pg.evaluate("document.dispatchEvent(new Event('visibilitychange'))")
            pg.wait_for_function("document.querySelectorAll('#cartbody .cprow').length>=3", timeout=30000)
            pg.click('.cprow:has-text("c.mzML")'); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=2", timeout=90000)
            assert pg.evaluate("window.RISCALDA.state().queued.indexOf('c.mzML')") == -1
            pg.click("#opbtn"); ready(pg); pg.wait_for_function("typeof E !== 'undefined' && E.files && E.files.length>=1", timeout=60000)
            hot["tic"] = pg.evaluate("(async()=>{const c=await getChrom(E.files[0].k,'tic',1);return JSON.stringify([Array.from(c.rt),Array.from(c.y)])})()")
        step("a click on a file still waiting takes it out of the queue", waiting)
        r.b.close()
        b2 = p.chromium.launch(); pc = b2.new_page(viewport={"width": 1500, "height": 2200}, reduced_motion="reduce"); pc.add_init_script(NOSTART)
        pc.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        def cold_same():
            pc.goto(f"http://127.0.0.1:{r.port}/"); pc.wait_for_selector("#pick", state="attached", timeout=60000); pc.wait_for_timeout(2500)
            pc.set_input_files("#pick", [str(D / "a.mzML")]); pc.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=90000)
            pc.click("#opbtn"); ready(pc); pc.wait_for_function("typeof E !== 'undefined' && E.files && E.files.length>=1", timeout=60000)
            assert pc.evaluate(SHA, "idx/a.mzML.mzidx") == hot["a.mzML"], "index bytes differ, cold against warm"
            tic = pc.evaluate("(async()=>{const c=await getChrom(E.files[0].k,'tic',1);return JSON.stringify([Array.from(c.rt),Array.from(c.y)])})()")
            assert tic == hot["tic"], "the TIC differs, cold against warm"
        step("cold open gives the same index bytes and the same TIC as the warm one", cold_same)
        b2.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
for s in steps: print("  ", s)
bad = [s for s in steps if s[1] != "ok"]
r.close() if hasattr(r, "srv") else None
sys.exit(1 if bad or any(e for e in r.errs if e[0] in ("pageerror",)) else 0)
