"""Folder pointed at and remembered (cartella-puntata.js), synthetic files, dev server. showDirectoryPicker is replaced by a folder of the browser's own private file
system (a REAL FileSystemDirectoryHandle: it is stored in IndexedDB and found again after a reload). Steps: grey rows (data / others), unknown files absent, a click loads one
(the row turns «loaded»), the handle is remembered across a reload, a file added to the folder shows up when the tab is shown again, the permission asked again with a click,
«Forget», and the message of a browser without the API. What a real folder of the disk adds (the picker window, FileSystemObserver on it) cannot be run in the cloud."""
import sys, os, base64, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="cartp_e2e_")); synth.write_full(D / "a.mzML", 5, seed=1); synth.write_full(D / "c.mzML", 15, seed=3)
A = base64.b64encode((D / "a.mzML").read_bytes()).decode(); C = base64.b64encode((D / "c.mzML").read_bytes()).decode()
INIT = """(() => { if (localStorage.getItem('nopicker')) { delete window.showDirectoryPicker; return; }
  window.showDirectoryPicker = async () => { const r = await navigator.storage.getDirectory(); return r.getDirectoryHandle('prova', { create: true }); };
  const q = FileSystemDirectoryHandle.prototype.queryPermission;
  FileSystemDirectoryHandle.prototype.queryPermission = async function (o) { return localStorage.getItem('perm') === 'prompt' ? 'prompt' : q.call(this, o); };
  FileSystemDirectoryHandle.prototype.requestPermission = async () => { localStorage.removeItem('perm'); return 'granted'; }; })()"""
PUT = """async ([path, b64, text]) => { let d = await (await navigator.storage.getDirectory()).getDirectoryHandle('prova', { create: true }); const parts = path.split('/'); const name = parts.pop();
  for (const p of parts) d = await d.getDirectoryHandle(p, { create: true });
  const w = await (await d.getFileHandle(name, { create: true })).createWritable(); await w.write(b64 ? Uint8Array.from(atob(b64), c => c.charCodeAt(0)) : text); await w.close(); }"""
def rows(pg): return pg.evaluate("Array.from(document.querySelectorAll('#cartbody .cprow .cpn')).map(e=>e.textContent)")
def count(pg): return pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length")
r = Run(port=8830 + 150, wd="/tmp/wd_cartp")
try:
    with sync_playwright() as p:
        # a persistent context: in a throw-away profile the cloud Chromium dies when a stored FileSystemDirectoryHandle is read back
        r.b = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="cartp_prof_"), viewport={"width": 1500, "height": 2200}, locale="it-IT", reduced_motion="reduce")
        pg = r.b.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('mzlab.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('mzlab.barra.hint','1');localStorage.setItem('mzlab.pannelli.affianca.hint','1');}catch(_){}"); pg.add_init_script(INIT)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#cartpick", timeout=60000)
        for a in (["a.mzML", A, None], ["sub/b.mzML", A, None], ["m.dam", None, "x"], ["l.msp", None, "Name: x\n"], ["t.csv", None, "a,b\n"], ["note.txt", None, "x"], ["sub/.hid.mzML", A, None]): pg.evaluate(PUT, a)
        def grey():
            pg.click("#cartpick"); pg.wait_for_selector("#cartbody .cprow")
            assert rows(pg) == ["a.mzML", "sub/b.mzML", "l.msp", "m.dam", "t.csv"], rows(pg)
            t = pg.inner_text("#cartbody"); assert "Dati" in t and "Altri" in t and "non caricato" in t, t
            assert pg.evaluate("getComputedStyle(document.querySelector('.cprow')).color") != pg.evaluate("getComputedStyle(document.body).color")
            assert "non caricato" in pg.get_attribute(".cprow", "aria-label")
            assert count(pg) == 0
        step("rows in grey (Data / Others), unknown and hidden files absent, nothing loaded", grey)
        def load():
            pg.click('.cprow:has-text("a.mzML")'); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=90000)
            pg.wait_for_function("document.querySelector('.cprow.done')"); assert "caricato" in pg.inner_text(".cprow.done")
        step("a click loads the file and its row turns «caricato»", load)
        def remembered():
            pg.reload(); pg.wait_for_selector("#cartbody .cprow", timeout=60000)
            assert "prova" in pg.inner_text("#cartbody") and rows(pg)[0] == "a.mzML", rows(pg)
        step("the folder is remembered after a reload", remembered)
        def added():
            pg.evaluate(PUT, ["c.mzML", C, None])
            pg.evaluate("document.dispatchEvent(new Event('visibilitychange'))")
            pg.wait_for_function("document.querySelectorAll('#cartbody .cprow').length>=6", timeout=30000)
            assert "c.mzML" in rows(pg), rows(pg)
        step("a file added to the folder appears when the tab is shown again", added)
        def table():
            pg.click('.cprow:has-text("t.csv")'); t = pg.inner_text("#cartinfo"); assert "tabella" in t and "pannello" in t, t
        step("a table explains where it is imported", table)
        def grant():
            pg.evaluate("localStorage.setItem('perm','prompt')"); pg.reload(); pg.wait_for_selector("#cartgrant", timeout=60000)
            assert not pg.query_selector("#cartbody .cprow")
            pg.click("#cartgrant"); pg.wait_for_selector("#cartbody .cprow", timeout=30000)
        step("permission asked again with one click", grant)
        def forget():
            pg.click("#cartforget"); pg.wait_for_selector("#cartforget", state="detached"); pg.reload(); pg.wait_for_selector("#cartpick", timeout=60000)
            assert not pg.query_selector("#cartbody .cprow")
        step("«Forget the folder» is kept across a reload", forget)
        def nosup():
            pg.evaluate("localStorage.setItem('nopicker','1')"); pg.reload(); pg.wait_for_selector("#cartpick", timeout=60000)
            assert not pg.query_selector("#cartnosup") and not pg.query_selector("#cartbody .cprow")
            with pg.expect_file_chooser() as fc: pg.click("#cartpick")
            assert fc.value.is_multiple(), "the icon falls back to the one-time folder picker"
        step("a browser without the API: the icon opens the one-time folder picker", nosup)
        r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300]))
for s in steps: print("  ", s)
r.report() if hasattr(r, "log") else None
bad = [s for s in steps if s[1] != "ok"]
sys.exit(1 if bad or any(e for e in r.errs if e[0] in ("pageerror",)) else 0)
