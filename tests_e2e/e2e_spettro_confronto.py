"""«Compare with another MS2 spectrum…» (spectrum menu, low resolution): the card takes two selections, asks the engine for each at nominal mass and shows the
`spectrum-compare/1` document: the cosine, the matched ions with both intensities, the ions of each side the other lacks, the collision-energy warning.
Two synthetic files of whole-mass centroids with a known cosine: A = 100/101/102 at 4/9/16 (CE 35), B = 100/101/103 at 4/9/25 (CE 20): 13 / sqrt(29 * 38) = 0.3916."""
# covers: mzlab/web/spettro-confronto.js, mzlab/web/libreria.js, mzlab/web/libreria-worker.js, mzlab/web/lang/en/spectrum-compare.js
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "confronto_tmp"; D.mkdir(parents=True, exist_ok=True)
FA, FB = D / "Sample_a.mzML", D / "Standard_b.mzML"
def product_ions(path, ce, peaks):
    """Three product-ion scans of precursor 253.1 at 1.0, 1.1 and 1.2 min with the same whole-mass centroids."""
    mz, it = np.array([p[0] for p in peaks], float), np.array([p[1] for p in peaks], float)
    rts = [1.0, 1.1, 1.2]
    ds._write(path, [ds._spec(i, rt, mz, it, level=2, prec=253.1, ce=ce) for i, rt in enumerate(rts)],
              [ds._chrom(0, "TIC", np.array(rts), np.full(3, it.sum())), ds._chrom(1, "BPC", np.array(rts), np.full(3, it.max()))])
product_ions(FA, 35.0, [(100.0, 4.0), (101.0, 9.0), (102.0, 16.0)])
product_ions(FB, 20.0, [(100.0, 4.0), (101.0, 9.0), (103.0, 25.0)])
SP2 = "E.panels.find(p => p.tab === 'ms2' && p.type === 'spec')"
def menu_entry(pg):
    c = pg.evaluate(f"(() => {{ const p = {SP2}; p.el.scrollIntoView({{block: 'center'}}); const q = p.cv.getBoundingClientRect(); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
    pg.mouse.click(c["x"], c["y"], button="right"); pg.wait_for_selector("#ctx >> text=Compare with another MS2 spectrum")
def card(pg):
    menu_entry(pg); pg.click("#ctx >> text=Compare with another MS2 spectrum"); wait_for_js(pg, "document.querySelector('#spcmp') && document.querySelector('#spcmp').open")
    assert pg.evaluate("document.querySelectorAll('#spcmp [data-s] select[data-f=file] option').length") == 4, "each side offers the two files"
    names = pg.evaluate("[...document.querySelectorAll('#spcmp [data-s]')].map(s => s.querySelector('select[data-f=file]').selectedOptions[0].textContent)")
    assert names[0] != names[1], names                                                   # B starts on the other file of the tab
    assert pg.evaluate("[...document.querySelectorAll('#spcmp [data-s] select[data-f=prec]')].map(s => s.value)") == ["253.1", "253.1"]
def compare(pg):
    for s in "ab":
        pg.fill(f"#spcmp [data-s={s}] [data-f=rt0]", "0.9"); pg.fill(f"#spcmp [data-s={s}] [data-f=rt1]", "1.3")
    pg.click("#spcmp-go"); wait_for_js(pg, "document.querySelector('#spcmp-out .cos')", 60000)
    t = pg.inner_text("#spcmp-out .cos"); assert "0.3916" in t, t
    rows = pg.evaluate("[...document.querySelectorAll('#spcmp-out table')].map(t => [...t.rows].map(r => [...r.cells].map(c => c.textContent.trim())))")
    side, matched, only_a, only_b = rows[0], rows[1], rows[2], rows[3]
    assert matched[1:] == [["100", "100", "4", "4"], ["101", "101", "9", "9"]], matched            # both intensities of each matched ion
    assert only_a[1:] == [["102", "16"]] and only_b[1:] == [["103", "25"]], (only_a, only_b)
    assert [r[3] for r in side[1:]] == ["35", "20"], side                                         # the collision energy of each
    w = pg.inner_text("#spcmp-out"); assert "35" in w and "20 eV" in w and "differ" in w, w       # the warning, from the catalogue
    assert "(default)" in pg.inner_text("#spcmp-out"), "the defaults used are shown"
def parameters(pg):
    """A parameter typed by the user goes to the engine and comes back in the document: tol 1 Da matches 102 with 103; ce_tol 20 eV silences the warning."""
    pg.fill("#spcmp-tol", "1"); pg.fill("#spcmp-cetol", "20"); pg.click("#spcmp-go")
    wait_for_js(pg, "document.querySelector('#spcmp-out .cos') && document.querySelector('#spcmp-out .cos').textContent.indexOf('0.3916') < 0", 60000)
    w = pg.inner_text("#spcmp-out"); assert "tolerance 1 Da" in w and "20 eV" in w and "differ" not in w, w
    assert pg.evaluate("document.querySelectorAll('#spcmp-out .warn').length") == 0
def empty(pg):
    """A window with no scan is an error the card says, not a blank card."""
    pg.fill("#spcmp-tol", ""); pg.fill("#spcmp-cetol", "")
    pg.fill("#spcmp [data-s=b] [data-f=rt0]", "50"); pg.fill("#spcmp [data-s=b] [data-f=rt1]", "60"); pg.click("#spcmp-go")
    wait_for_js(pg, "document.querySelector('#spcmp-msg').classList.contains('err') && document.querySelector('#spcmp-msg').textContent.trim() !== ''", 60000)
    assert pg.evaluate("document.querySelectorAll('#spcmp-out .cos').length") == 0
def run(port, wd, body, size=None):
    r = Run(port=port, wd=wd)
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            if size: pg.set_viewport_size(size)
            stage(pg, [str(FA), str(FB)]); pg.click("text=Load data"); ready(pg)
            open_ms2(pg); pg.locator('#flst [data-pg]').first.dblclick(); wait_for_js(pg, f"!!{SP2}"); ready(pg)
            body(pg)
            errs = [e for e in r.errs if e[0] == "pageerror"]; assert not errs, errs
    finally: r.close()
def whole(pg):
    card(pg); compare(pg); parameters(pg); empty(pg)
    assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"), "horizontal page scroll"
step("menu → card → cosine 0.3916, matched ions with both intensities, the ions of each side, the CE warning, the parameters, an empty window", lambda: run(8931, "/tmp/wd_spc1", whole))
def narrow(pg):
    """The context menu is a right click (the phone has long press): the card is opened as the menu entry does, and must fit the width."""
    pg.evaluate(f"loadScript('m:spettro-confronto.js').then(() => SPCMP.open({SP2}))"); wait_for_js(pg, "document.querySelector('#spcmp') && document.querySelector('#spcmp').open")
    compare(pg)
    assert pg.evaluate("document.querySelector('#spcmp').getBoundingClientRect().right <= window.innerWidth + 1"), "the card is wider than the page"
step("phone width: the card fits", lambda: run(8932, "/tmp/wd_spc2", narrow, {"width": 390, "height": 800}))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
