"""Loading screen: with a folder pointed at the primary button «Carica i file» is right under it (off until a file is chosen, then it opens the files); the same file
(name and size) chosen again, by click on the folder row, by the file picker or by drop, is not loaded twice: a short notice and its row lights up."""
import sys, os, base64, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
import synth
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:300]))
D = Path(tempfile.mkdtemp(prefix="cartgo_e2e_")); synth.write_full(D / "a.mzML", 5, seed=1)
A = base64.b64encode((D / "a.mzML").read_bytes()).decode()
INIT = """(() => { window.showDirectoryPicker = async () => { const r = await navigator.storage.getDirectory(); return r.getDirectoryHandle('provago', { create: true }); }; })()"""
PUT = """async ([path, b64]) => { const d = await (await navigator.storage.getDirectory()).getDirectoryHandle('provago', { create: true });
  const w = await (await d.getFileHandle(path, { create: true })).createWritable(); await w.write(Uint8Array.from(atob(b64), c => c.charCodeAt(0))); await w.close(); }"""
count = lambda pg: pg.evaluate("document.querySelectorAll('#flist input[data-k=use]').length")
r = Run(port=8981, wd="/tmp/wd_cartgo")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch_persistent_context(tempfile.mkdtemp(prefix="cartgo_prof_"), viewport={"width": 1280, "height": 1400}, locale="it-IT", reduced_motion="reduce")
        pg = r.b.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}"); pg.add_init_script(INIT)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_selector("#cartpick", timeout=60000)
        pg.evaluate(PUT, ["a.mzML", A])
        def button():
            pg.click("#cartpick"); pg.wait_for_selector("#cartbody .cprow")
            assert pg.is_visible("#cartgo") and pg.is_disabled("#cartgo"), "off until a file is chosen"
            assert pg.inner_text("#cartgo") == "Carica i file"
            b, l = pg.locator("#cartgo").bounding_box(), pg.locator("#cartbody .cprow").first.bounding_box()
            assert b["y"] < l["y"], (b, l)                                  # above the list, right under the folder line
            pg.click('.cprow:has-text("a.mzML")'); pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=90000)
            pg.wait_for_function("!document.querySelector('#cartgo').disabled")
            pg.click("#cartgo"); ready(pg); assert pg.evaluate("E.files.length") >= 1
        step("the primary button is under the folder, off until a file is chosen, and opens the files", button)
        br2 = p.chromium.launch(); pg2 = br2.new_page(viewport={"width": 1280, "height": 1400}, locale="it-IT"); pg2.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        pg2.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg2.goto(f"http://127.0.0.1:{r.port}/"); pg2.wait_for_selector("#drop", timeout=60000); pg2.wait_for_function("window.MZLAB_LR !== undefined", timeout=60000)
        def twice():
            pg2.set_input_files("#pick", [str(D / "a.mzML")]); pg2.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length===1", timeout=90000)
            pg2.set_input_files("#pick", [str(D / "a.mzML")]); pg2.wait_for_function("document.querySelector('#cartinfo') && /già caricato/.test(document.querySelector('#cartinfo').textContent)", timeout=30000)
            assert count(pg2) == 1, count(pg2)
            pg2.wait_for_selector("#flist tr.dupflash", timeout=3000)
            pg2.set_input_files("#pick", [str(D / "a.mzML"), str(D / "a.mzML")]); pg2.wait_for_timeout(1500); assert count(pg2) == 1, count(pg2)
        step("the same file chosen again is ignored: notice «già caricato», its row lights up, no duplicate", twice)
        def fast():
            pg2.evaluate("document.querySelector('#flist [data-rm]').click()"); pg2.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length===0", timeout=30000)
            pg2.evaluate("""(async () => { const f = new File([new Uint8Array(await (await fetch('static/esempi/FullScan_t0.mzML')).arrayBuffer())], 'x_t0.mzML'); window.__ups = [upload([f], 'data'), upload([f], 'data')]; await Promise.all(window.__ups); })()""")
            pg2.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=1", timeout=90000); pg2.wait_for_timeout(1500)
            assert count(pg2) == 1, count(pg2)
        step("two quick requests for the same file read it once", fast)
        r.b.close()
finally:
    pass
for s in steps: print(s)
print("ERRORS", r.errs)
assert steps and all(s[1] == "ok" for s in steps) and not r.errs
