"""The × of the Sample table and the × of the side panel are ONE action: the file leaves the side panel, the panels follow the engine's new indexes (the other
files still draw) and Undo brings it back. Synthetic product-ion mzML files only; the DAD file also checks that its optical spectra raise no «partial read» notice."""
# covers: mzlab/web/lista-file.js, mzlab/web/shell.js, mzlab/web/grafici.js, mzlab/web/pannelli.js
import sys, os, tempfile; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pathlib import Path
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
tmp = Path(tempfile.mkdtemp(prefix="synth_ms2_"))
NAMES = ["cbz_a.mzML", "cbz_b.mzML", "cbz_c.mzML"]
for i, n in enumerate(NAMES): ds.ms2(tmp / n, np.random.default_rng(10 + i), t=5 + 10 * i)
ds.full_scan(tmp / "uv_d.mzML", 0, np.random.default_rng(3), dad=True)
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e:
        import traceback; ln = [f.lineno for f in traceback.extract_tb(e.__traceback__) if f.filename.endswith("e2e_togli_tabella.py")][-1]
        steps.append((name, f"FAIL line {ln}: " + " ".join(str(e).split())[:400]))
WORK = "document.getElementById('v-workspace').dataset.work"
live = "E.files.filter(f => !f.gone)"
r = Run(port=8968, wd="/tmp/wd_togli_tabella")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1440, "height": 900})
        pg.wait_for_selector("#drop"); wait_for_js(pg, "window.qqReady === true", 60000)
        pg.set_input_files("#pick", [str(tmp / n) for n in NAMES])
        wait_for_js(pg, f"UPBUSY === 0 && OPENNOW.busy() === 0 && {live}.length === 3", 90000)
        pg.wait_for_selector(".pnl canvas", timeout=60000); ready(pg)
        first = pg.evaluate(f"{live}[0].file.split('#')[0]")
        mod = "Meta" if pg.evaluate("/Mac/.test(navigator.platform)") else "Control"
        def side_open():
            if not pg.evaluate("document.querySelector('#v-data').classList.contains('sb-open')"): pg.click("#sb-toggle"); wait_for_js(pg, "document.querySelector('#v-data').classList.contains('sb-open')")
        def unfold():          # the group of the experiment type may be folded: a user opens it with its header
            if pg.evaluate("getComputedStyle(document.querySelector('#flst .grp-files')).display") == "none": pg.click("#flst .fgh >> nth=0")
        def remove_via_table():
            pg.evaluate("document.querySelector('#addf').click()"); wait_for_js(pg, f"{WORK} === 'add'")
            i = pg.evaluate(f"ST.files.findIndex(f => f.name === {first!r})"); assert i >= 0, "the row of the first file"
            pg.click(f'#flist [data-rm="{i}"]'); wait_for_js(pg, f"{live}.length === 2", 30000)
            pg.click("#backwork"); wait_for_js(pg, f"{WORK} === 'panels'"); side_open()
        def table_x():
            remove_via_table()
            assert pg.evaluate(f"E.files.filter(f => f.gone && f.file.split('#')[0] === {first!r}).length") == 1, "marked gone (the same path as the side panel ×)"
            wait_for_js(pg, f"!document.querySelector('#flst').innerText.includes({first!r})", 20000)
            assert pg.locator("#flst .fl:not(.pr)").count() == 2, "the side panel lists the two files left"
        step("the table × takes the file out of the side panel", table_x)
        def undo():
            # the view changes after the removal («Back to work!», the side panel) are steps of the same stack: Undo walks back to the removal
            for _ in range(6):
                if pg.evaluate(f"{live}.length") == 3: break
                pg.click("#np-undo")
                try: wait_for_js(pg, f"{live}.length === 3", 800)
                except Exception: pass          # an earlier view step was undone: the next click goes one step further back
            wait_for_js(pg, f"{live}.length === 3", 10000)
            assert pg.locator("#flst .fl:not(.pr)").count() == 3, "the side panel lists the file again"
        step("Undo brings the file back", undo)
        def others_draw():
            remove_via_table()
            wait_for_js(pg, "E.files.length === 2", 40000)          # the Undo window closed: the engine dropped the file and its indexes shifted
            for k in range(2):
                unfold()
                pg.click(f"#flst .fl:not(.pr) .nm >> nth={k}"); ready(pg)
                assert pg.locator(".pnl canvas").count() >= 1, "a panel draws"
                bad = pg.evaluate("E.panels.filter(p => p.type !== 'xic' && p.k != null && (!E.files[p.k] || E.files[p.k].gone)).length"); assert bad == 0, f"{bad} panels point to a gone or missing file"
                assert pg.evaluate("E.files.every((f, i) => f.k === i)"), "the indexes follow the engine"
        step("after the engine re-indexes, the other files still draw", others_draw)
        def transitions_grouped():
            pg.wait_for_selector("select[data-o=tr]", timeout=30000)
            g = pg.evaluate("[...document.querySelectorAll('select[data-o=tr] optgroup')].map(o => [o.label, o.querySelectorAll('option').length])")
            assert g and all(l.startswith("Fragments of precursor ") for l, _ in g), g
            assert pg.evaluate("document.querySelector('select[data-o=tr] option').textContent") == "all MS2 scans", "the whole-scan view stays first"
            assert len(g) == 2, f"one group per acquired precursor (229.1, 305.0): {g}"
        step("the transition menu groups the fragments by precursor, «all MS2 scans» first", transitions_grouped)
        def no_false_notice():
            pg.set_input_files("#pick", [str(tmp / "uv_d.mzML")]); wait_for_js(pg, "UPBUSY === 0 && OPENNOW.busy() === 0", 90000)
            txt = pg.evaluate("document.body.innerText"); assert "partial read" not in txt.lower() and "file ends before" not in txt.lower(), "no truncation notice for optical spectra"
        step("a file with optical (DAD) spectra raises no partial-read notice", no_false_notice)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
r.report()
