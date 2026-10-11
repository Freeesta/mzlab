"""Conversion of .raw files with a simulated engine: the progress panel (file i of n, phase, progressbar), time filled from the .raw name, downloads of the mzML."""
import sys, os, json; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
from synth import write_full
from pathlib import Path
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_conversione.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + str(e).split("\n")[0][:240]))
D = Path("/tmp/wd_conv_files"); D.mkdir(parents=True, exist_ok=True)
write_full(D / "x.mzML", 0, seed=3); XML = (D / "x.mzML").read_text(encoding="utf-8")
for n in ("irr_t5.raw", "irr_t10.raw"): (D / n).write_bytes(b"RAW" * 10)
FAKE = """window.MZLAB_RUST_READY = Promise.resolve({ convertRaw: async (f, onp) => {
  const xml = %s;
  window.__log = (window.__log || []).concat([f.name]);
  onp({ total: 100, read: 0, written: 0 });
  await new Promise(r => { window.__go = r; });
  onp({ total: 100, read: 40, written: 5000 });
  await new Promise(r => { window.__go = r; });
  onp({ total: 100, read: 100, written: 9000 });
  onp({ total: 1, read: 1, written: 9000, copied: 4500, size: 9000 });
  await new Promise(r => { window.__go = r; });
  return new Blob([xml], { type: 'application/xml' });
} });""" % json.dumps(XML)
r = Run(port=8924, wd="/tmp/wd_conv")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1500, "height": 2000}, accept_downloads=True)
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));}catch(_){}")
        pg.add_init_script(FAKE)
        pg.goto(f"http://127.0.0.1:{r.port}/"); pg.wait_for_timeout(800)
        pg.set_input_files("#pick", [str(D / "irr_t5.raw"), str(D / "irr_t10.raw")])
        def progress():
            pg.wait_for_selector(".convprog"); pg.wait_for_function("!!window.__go")
            assert "1 di 2: irr_t5.raw" in pg.inner_text(".convprog"), pg.inner_text(".convprog")
            assert pg.get_attribute(".convprog [role=progressbar]", "aria-valuenow") == "0"
            pg.evaluate("window.__go(); window.__go=null"); pg.wait_for_function("!!window.__go")
            t = pg.inner_text(".convprog")
            assert "conversione in mzML" in t and "40%" in t and "MB" in t, t
            assert pg.get_attribute(".convprog [role=progressbar]", "aria-valuenow") == "40"
            pg.evaluate("window.__go(); window.__go=null"); pg.wait_for_function("!!window.__go")
            assert "scrittura" in pg.inner_text(".convprog") and pg.get_attribute(".convprog [role=progressbar]", "aria-valuenow") == "50"
            pg.evaluate("window.__go(); window.__go=null")
        step("the panel says 1 di 2, the phase, the percent and the bar moves", progress)
        def second():
            pg.wait_for_function("document.querySelector('.convprog b') && document.querySelector('.convprog b').textContent.includes('2 di 2')")
            for _ in range(3): pg.wait_for_function("!!window.__go"); pg.evaluate("window.__go(); window.__go=null")
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=2", timeout=60000)
            assert pg.evaluate("document.querySelectorAll('.convprog').length") == 0
        step("the second file follows and the panel goes away", second)
        def table():
            names = pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")
            assert names == ["irr_t5.mzML", "irr_t10.mzML"], names
            assert pg.evaluate("Array.from(document.querySelectorAll('#flist input[data-k=time]')).map(i=>i.value)") == ["5", "10"]
        step("the time comes from the .raw name and the order is by time", table)
        def downloads():
            assert pg.locator("#flist [data-dl]").count() == 2 and pg.is_visible("#dlall")
            with pg.expect_download() as d: pg.click("#flist [data-dl='irr_t5.mzML']")
            dl = d.value; assert dl.suggested_filename == "irr_t5.mzML"
            assert Path(dl.path()).read_text(encoding="utf-8") == XML
            with pg.expect_download() as d2: pg.click("#dlall")
            assert d2.value.suggested_filename == "irr_t5.mzML"
        step("Scarica mzML per file and Scarica tutti give the converted mzML", downloads)
        def opened():
            pg.click("text=Carica dati"); ready(pg)
            assert pg.evaluate("E.files.length") == 2
            assert pg.locator("#flst [data-dl]").count() == 2 and pg.locator("#flst [data-dl-all]").count() == 1
            with pg.expect_download() as d: pg.evaluate("document.querySelector('#flst [data-dl]').click()")      # the side bar may be folded: the click goes to the button itself
            assert d.value.suggested_filename.endswith(".mzML")
        step("the file list has the buttons too", opened)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
