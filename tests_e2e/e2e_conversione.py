"""Conversion of .raw files with a simulated engine: the progress panel (file i of n, phases 1/3 and 2/3 in order, progressbar and total bar that move, heartbeat after 2 s, cancel), time filled from the .raw name, downloads of the mzML."""
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
write_full(D / "dup.mzML", 0, seed=4); (D / "dup.raw").write_bytes(b"RAW" * 10)
for n in ("irr_t5.raw", "irr_t10.raw"): (D / n).write_bytes(b"RAW" * 10)
FAKE = """window.MZLAB_RUST_READY = Promise.resolve({ convertRaw: async (f, onp, signal, opts) => {
  const xml = %s;
  const wait = () => new Promise((r, j) => { window.__go = r; if (signal) signal.addEventListener('abort', () => j(Object.assign(new Error('cancelled'), { cancelled: true })), { once: true }); });
  window.__log = (window.__log || []).concat([f.name]);
  window.__prof = (window.__prof || []).concat([!!(opts && opts.profile)]);
  onp({ total: 100, read: 100, written: 0, scans: 0, scansTotal: null });      // the container is open (its tail was read: read = total) but no scan has been converted
  await wait();
  onp({ total: 100, read: 100, written: 0, scans: 0, scansTotal: 10 });
  await wait();
  onp({ total: 100, read: 100, written: 5000, scans: 4, scansTotal: 10 });
  await wait();
  onp({ total: 100, read: 100, written: 9000, scans: 10, scansTotal: 10 });
  onp({ total: 1, read: 1, written: 9000, copied: 4500, size: 9000 });
  await wait();
  return new Blob([xml], { type: 'application/xml' });
} });""" % json.dumps(XML)
r = Run(port=8924, wd="/tmp/wd_conv")
try:
    with sync_playwright() as p:
        r.b = p.chromium.launch()
        ctx = r.b.new_context(viewport={"width": 1500, "height": 2000}, accept_downloads=True)
        pg = ctx.new_page(); pg.on("pageerror", lambda e: r.errs.append(("pageerror", str(e))))
        pg.add_init_script("try{localStorage.setItem('qqq.tour.lr',JSON.stringify({stato:'saltato'}));localStorage.setItem('qqq.barra.hint','1');}catch(_){}")
        def fake():          # the engine of the site is there; the simulated one replaces its .raw converter (the conversion, not the reading of the mzML); a reload brings the real one back
            pg.wait_for_selector("#rawprofile", state="attached"); pg.evaluate("window.MZLAB_RUST_READY"); pg.evaluate(FAKE); pg.wait_for_timeout(300)
        pg.goto(f"http://127.0.0.1:{r.port}/"); fake()
        pg.set_input_files("#pick", [str(D / "irr_t5.raw"), str(D / "irr_t10.raw")])
        def go(): pg.evaluate("window.__go(); window.__go=null"); 
        def progress():
            pg.wait_for_selector(".convprog"); pg.wait_for_function("!!window.__go")
            t = pg.inner_text(".convprog")
            assert "File 1 di 2: irr_t5.raw" in t and "Conversione in mzML" in t and "/3" not in t and "restano" not in t, t       # no "1/3", no estimate; read = total at once, yet no percent: it comes from the scans
            assert pg.get_attribute(".convprog [data-r=bar]", "aria-valuenow") is None
            go(); pg.wait_for_function("!!window.__go")
            t = pg.inner_text(".convprog")
            assert "Conversione in mzML" in t and "0%" in t and "0 di 10 scansioni" in t, t
            assert pg.get_attribute(".convprog [data-r=bar]", "aria-valuenow") == "0"
            go(); pg.wait_for_function("!!window.__go")
            t = pg.inner_text(".convprog")
            assert "40%" in t and "4 di 10 scansioni" in t and "MB scritti" in t and "trascorso" in t, t
            assert pg.get_attribute(".convprog [data-r=bar]", "aria-valuenow") == "15", pg.get_attribute(".convprog [data-r=bar]", "aria-valuenow")      # ONE bar: 0.4 of the first of two files = 20% of the conversion = 15% of the job (the conversion is 3/4 of it)
            assert pg.locator(".convprog [data-r=tot]").count() == 0 and "restano" not in t
            assert pg.evaluate("getComputedStyle(document.querySelector('.convprog [data-r=detail]')).fontVariantNumeric").find("tabular-nums") >= 0
            assert pg.locator(".cp-spin").count() == 0
            pg.wait_for_function("document.querySelector('.cp-beat') && document.querySelector('.cp-beat').textContent.includes('Sto ancora convertendo')", timeout=8000)    # no news for 2 s: still working
            assert pg.locator(".cp-spin").count() == 1 and pg.get_attribute("[data-r=beat]", "aria-live") == "polite"
            go(); pg.wait_for_function("!!window.__go")
            assert pg.get_attribute(".convprog [data-r=bar]", "aria-valuenow") == "37"      # the hand-over of the mzML: the first of two files is converted
            assert pg.locator(".cp-spin").count() == 0
            go()
        step("the panel says File 1 di 2, the phase, the real percent, one bar over the job, no time left and the heartbeat", progress)
        def second():
            pg.wait_for_function("document.querySelector('.convprog b') && document.querySelector('.convprog b').textContent.includes('2 di 2')")
            pg.wait_for_function("!!window.__go"); assert pg.get_attribute(".convprog [data-r=bar]", "aria-valuenow") == "37"      # the first file is done
            for _ in range(4): pg.wait_for_function("!!window.__go"); pg.evaluate("window.__go(); window.__go=null")
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
        def same_base():
            pg.set_input_files("#pick", [str(D / "dup.raw"), str(D / "dup.mzML")])
            for _ in range(4): pg.wait_for_function("!!window.__go", timeout=60000); pg.evaluate("window.__go(); window.__go=null")
            pg.wait_for_function("document.querySelectorAll('#flist input[data-k=use]').length>=4", timeout=60000)
            names = pg.evaluate("Array.from(document.querySelectorAll('#flist tr:not(.grp)')).slice(1).map(tr=>tr.children[1].textContent.trim().split(' ')[0])")
            assert "dup.mzML" in names and "dup.raw.mzML" in names, names
        step("a .raw and an mzML with the same base name are both listed", same_base)
        def cancel():
            for n in ("c1.raw", "c2.raw"): (D / n).write_bytes(b"RAW" * 10)
            pg.evaluate("window.__log=[]")
            pg.set_input_files("#pick", [str(D / "c1.raw"), str(D / "c2.raw")])
            pg.wait_for_selector(".convprog [data-r=cancel]", state="attached"); pg.wait_for_function("!!window.__go")
            pg.evaluate("document.querySelector('.convprog [data-r=cancel]').click()")      # the panel is on the loading screen, which the viewer hides
            pg.wait_for_function("document.querySelectorAll('.convprog').length==0", timeout=10000)
            assert pg.evaluate("window.__log") == ["c1.raw"], pg.evaluate("window.__log")      # the second .raw was not started
            assert "annullata" in pg.evaluate("document.getElementById('up').textContent")
        step("Annulla stops the conversion and leaves the other .raw out", cancel)
        def profile():
            assert pg.evaluate("document.getElementById('rawprof').hidden") is False
            assert pg.evaluate("document.getElementById('rawprofile').checked") is False      # not implicit: the default is the centroids
            assert "Conserva i dati in profilo (predefinito: centroidi)" in pg.inner_text("#rawprof") and ".wiff" in pg.inner_text("#rawprofnote")      # one option for every format that has both
            cs = pg.evaluate("(() => { const l = getComputedStyle(document.querySelector('#rawprof .convopt-l')), c = getComputedStyle(document.getElementById('rawprofile')); return [l.display, l.alignItems, c.width, c.height]; })()")
            assert cs == ["inline-flex", "center", "16px", "16px"], cs      # the box is centred on the text and big enough to hit
            pg.evaluate("window.__prof=[]; window.__log=[]")
            (D / "p1.raw").write_bytes(b"RAW" * 10)
            pg.set_input_files("#pick", [str(D / "p1.raw")]); pg.wait_for_function("!!window.__go")
            assert pg.evaluate("window.__prof") == [False]
            pg.evaluate("document.querySelector('.convprog [data-r=cancel]').click()")
            pg.wait_for_function("document.querySelectorAll('.convprog').length==0", timeout=10000)
            pg.evaluate("document.getElementById('rawprofile').click()")
            assert pg.evaluate("localStorage.getItem('qqq.rawprofile')") == "1"
            pg.reload(); fake()
            assert pg.evaluate("document.getElementById('rawprofile').checked") is True      # remembered
            pg.evaluate("window.__prof=[]")
            pg.set_input_files("#pick", [str(D / "p1.raw")]); pg.wait_for_function("!!window.__go")
            assert pg.evaluate("window.__prof") == [True]
            pg.evaluate("document.querySelector('.convprog [data-r=cancel]').click()")
        step("Profilo / Centroide: centroids by default, the choice is remembered and reaches the converter", profile)
finally:
    r.close()
for s in steps: print(s)
print("ERRORS", [e for e in r.errs if e[0] != "console.warning"][:5])
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
