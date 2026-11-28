"""«Compare with another MS2 spectrum…» (spectrum menu, low resolution): the card takes two selections, asks the engine for each at nominal mass and shows the
`spectrum-compare/1` document: the cosine, the matched ions with both intensities, the ions of each side the other lacks, the collision-energy warning.
Two synthetic files of whole-mass centroids with a known cosine: A = 100/101/102 at 4/9/16 (CE 35), B = 100/101/103 at 4/9/25 (CE 20): 13 / sqrt(29 * 38) = 0.3916.
A third file holds A's ions shifted by +0.55 Da (a miscalibrated standard) and an MS1 ion at 200.55: without an offset its bins miss A's (cosine 18 / 29 = 0.6207), with the offset
of 0.55 Da, typed or measured with auto:200@0.5, they are the same (1.0000)."""
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
FA, FB, FC = D / "Sample_a.mzML", D / "Standard_b.mzML", D / "Shifted_c.mzML"
def product_ions(path, ce, peaks):
    """Three product-ion scans of precursor 253.1 at 1.0, 1.1 and 1.2 min with the same whole-mass centroids."""
    mz, it = np.array([p[0] for p in peaks], float), np.array([p[1] for p in peaks], float)
    rts = [1.0, 1.1, 1.2]
    ds._write(path, [ds._spec(i, rt, mz, it, level=2, prec=253.1, ce=ce) for i, rt in enumerate(rts)],
              [ds._chrom(0, "TIC", np.array(rts), np.full(3, it.sum())), ds._chrom(1, "BPC", np.array(rts), np.full(3, it.max()))])
product_ions(FA, 35.0, [(100.0, 4.0), (101.0, 9.0), (102.0, 16.0)])
product_ions(FB, 20.0, [(100.0, 4.0), (101.0, 9.0), (103.0, 25.0)])
SHIFT = 0.55
def shifted(path):
    """A's ions + 0.55 Da, and nine MS1 scans of a known ion at 200 + 0.55 (a clear peak at 0.5 min) for `auto:200@0.5`."""
    rts = [0.1 + 0.1 * i for i in range(9)]                                               # MS1 first (peak at 0.5 min), then the MS2 scans, indices in time order
    ms1 = [ds._spec(i, rt, np.array([200.0 + SHIFT]), np.array([y]), level=1) for i, (rt, y) in enumerate(zip(rts, [1, 1, 1, 100, 1000, 100, 1, 1, 1]))]
    ms2 = [ds._spec(9 + i, rt, np.array([100.0, 101.0, 102.0]) + SHIFT, np.array([4.0, 9.0, 16.0]), level=2, prec=253.1, ce=35.0) for i, rt in enumerate([1.0, 1.1, 1.2])]
    ds._write(path, ms1 + ms2, [ds._chrom(0, "TIC", np.array(rts), np.full(9, 100.0))])
shifted(FC)
SP2 = "E.panels.find(p => p.tab === 'ms2' && p.type === 'spec')"
def menu_entry(pg):
    c = pg.evaluate(f"(() => {{ const p = {SP2}; p.el.scrollIntoView({{block: 'center'}}); const q = p.cv.getBoundingClientRect(); return {{x: q.left + q.width * 0.5, y: q.top + q.height * 0.3}}; }})()")
    pg.mouse.click(c["x"], c["y"], button="right"); pg.wait_for_selector("#ctx >> text=Compare with another MS2 spectrum")
def card(pg):
    menu_entry(pg); pg.click("#ctx >> text=Compare with another MS2 spectrum"); wait_for_js(pg, "document.querySelector('#spcmp') && document.querySelector('#spcmp').open")
    assert pg.evaluate("document.querySelectorAll('#spcmp [data-s] select[data-f=file] option').length") == 6, "each side offers the three files"
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
def mirror(pg):
    """The canvas draws A up and B down: the two matched ions (100, 101) as joined bars of one colour, 102 only above and 103 only below in the other; the data table lists them."""
    bars = pg.evaluate("JSON.parse(document.querySelector('#spcmp-cv').dataset.bars)")
    assert sorted(b for b in bars if b[0] == "a") == [["a", 100, 0.25, 1], ["a", 101, 0.5625, 1], ["a", 102, 1, 0]], bars     # share of A's base peak (16)
    assert sorted(b for b in bars if b[0] == "b") == [["b", 100, 0.16, 1], ["b", 101, 0.36, 1], ["b", 103, 1, 0]], bars        # share of B's base peak (25)
    # the pixels: the middle of each bar has the colour of its kind (matched blue, unmatched vermilion), the matched pair joins both sides, the page background is elsewhere
    px = pg.evaluate("""(() => { const cv = document.querySelector('#spcmp-cv'), m = cv._mg, g = cv.getContext('2d'), d = cv.width / cv.clientWidth, at = (mz, up, rel) => [...g.getImageData(Math.round(m.X(mz) * d), Math.round((m.mid - (up ? 1 : -1) * rel * m.half / 2) * d), 1, 1).data.slice(0, 3)];
      return { m101: at(101, true, 0.5625), m101b: at(101, false, 0.36), a102: at(102, true, 1), b103: at(103, false, 1), empty: at(110, true, 1) }; })()""")
    blue = lambda c: c[2] > c[0] + 80                                                       # Okabe-Ito blue; the thin bars are anti-aliased, so the hue is compared, not the exact value
    red = lambda c: c[0] > c[2] + 80
    assert blue(px["m101"]) and blue(px["m101b"]), px                                       # matched: the same colour on both sides
    assert red(px["a102"]) and red(px["b103"]), px                                          # only in one spectrum: the other colour
    assert px["empty"] == [255, 255, 255] or not (blue(px["empty"]) or red(px["empty"])), px
    pg.focus("#spcmp-cv"); pg.keyboard.press("t"); wait_for_js(pg, "document.querySelector('#tdati[open] table')")
    t = pg.inner_text("#tdati table")
    assert "Matched" in t and "103" in t and "A" in t and "B" in t, t
    pg.keyboard.press("Escape")
    assert pg.evaluate("document.querySelector('#spcmp').open"), "Escape closes the table, not the card"
def parameters(pg):
    """A parameter typed by the user goes to the engine and comes back in the document: tol 1 Da matches 102 with 103; ce_tol 20 eV silences the warning."""
    pg.fill("#spcmp-tol", "1"); pg.fill("#spcmp-cetol", "20"); pg.click("#spcmp-go")
    wait_for_js(pg, "document.querySelector('#spcmp-out .cos') && document.querySelector('#spcmp-out .cos').textContent.indexOf('0.3916') < 0", 60000)
    w = pg.inner_text("#spcmp-out"); assert "tolerance 1 Da" in w and "20 eV" in w and "differ" not in w, w
    assert pg.evaluate("document.querySelectorAll('#spcmp-out .warn').length") == 0
def side_offset(pg, s, text):
    want = "Shifted_c" if s == "b" else "Sample_a"                                           # the option text may carry the extension
    v = pg.evaluate(f"[...document.querySelectorAll('#spcmp [data-s={s}] [data-f=file] option')].find(o => o.textContent.includes('{want}')).value")
    pg.select_option(f"#spcmp [data-s={s}] [data-f=file]", value=v)
    pg.fill(f"#spcmp [data-s={s}] [data-f=off]", text)
def offsets(pg):
    """The offset of a side goes to its route (`mzoff=`) and comes back as evidence: none leaves the query and the cosine as they were; 0.55 aligns the bins; auto measures it."""
    # the route is answered in the page (no network request to see): record the queries the card hands to fetch
    pg.evaluate("(() => { window.__q = []; const f0 = window.fetch; window.fetch = function (u, init) { if (typeof u === 'string' && u.includes('api/spectrum')) window.__q.push(u); return f0.call(window, u, init); }; })()")
    queries = lambda: pg.evaluate("window.__q")
    for s in "ab":
        pg.fill(f"#spcmp [data-s={s}] [data-f=rt0]", "0.9"); pg.fill(f"#spcmp [data-s={s}] [data-f=rt1]", "1.3")
    side_offset(pg, "a", ""); side_offset(pg, "b", "")
    pg.fill("#spcmp-tol", ""); pg.fill("#spcmp-cetol", "")
    def run_cos(want):
        pg.evaluate("document.querySelector('#spcmp-out').innerHTML = ''"); pg.click("#spcmp-go")
        wait_for_js(pg, "document.querySelector('#spcmp-out .cos')", 60000)
        t = pg.inner_text("#spcmp-out .cos"); assert want in t, (want, t)
    run_cos("0.6207"); assert queries() and not any("mzoff" in u for u in queries()), queries()                # no offset: the query and the bins are the ones of before
    assert "0 Da" in pg.inner_text("#spcmp-offs"), pg.inner_text("#spcmp-offs")                # the evidence is there for 0 too
    pg.evaluate("window.__q = []"); pg.fill("#spcmp [data-s=b] [data-f=off]", "0.55"); run_cos("1.0000")
    assert sum("mzoff=0.55" in u for u in queries()) == 1, queries()                                       # only side B asked for it
    ev = pg.inner_text("#spcmp-offs"); assert "0.55 Da" in ev and "B" in ev, ev
    pg.fill("#spcmp [data-s=b] [data-f=off]", "auto:200@0.5"); run_cos("1.0000")
    ev = pg.inner_text("#spcmp-offs"); assert "0.55 Da" in ev and "m/z 200" in ev and "found" in ev and "3 scans" in ev, ev   # auto: the ion, the scans, the status
    pg.fill("#spcmp [data-s=b] [data-f=off]", "auto:300@0.5"); run_cos("0.6207")
    ev = pg.inner_text("#spcmp-offs"); assert "not found" in ev and "no offset was applied" in ev, ev   # an ion not found says so, with the engine's warning, and applies 0
    pg.fill("#spcmp [data-s=b] [data-f=off]", "x1"); pg.click("#spcmp-go")
    wait_for_js(pg, "document.querySelector('#spcmp-msg').classList.contains('err') && document.querySelector('#spcmp-msg').textContent.trim() !== ''", 60000)
    m = pg.inner_text("#spcmp-msg"); assert "x1" in m, ("the route refusal of a bad offset reaches the card", m)
    pg.fill("#spcmp [data-s=b] [data-f=off]", "")
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
            stage(pg, [str(FA), str(FB), str(FC)]); pg.click("text=Load data"); ready(pg)
            open_ms2(pg); pg.locator('#flst [data-pg]').first.dblclick(); wait_for_js(pg, f"!!{SP2}"); ready(pg)
            body(pg)
            errs = [e for e in r.errs if e[0] == "pageerror"]; assert not errs, errs
    finally: r.close()
def whole(pg):
    card(pg); compare(pg); mirror(pg); parameters(pg); offsets(pg); empty(pg)
    assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"), "horizontal page scroll"
step("menu → card → cosine 0.3916, matched ions with both intensities, the mirror canvas and its table, the ions of each side, the CE warning, the parameters, an m/z offset per side (fixed, auto, not found, refused), an empty window", lambda: run(8931, "/tmp/wd_spc1", whole))
def narrow(pg):
    """The context menu is a right click (the phone has long press): the card is opened as the menu entry does, and must fit the width."""
    pg.evaluate(f"loadScript('m:spettro-confronto.js').then(() => SPCMP.open({SP2}))"); wait_for_js(pg, "document.querySelector('#spcmp') && document.querySelector('#spcmp').open")
    compare(pg)
    assert pg.evaluate("document.querySelector('#spcmp').getBoundingClientRect().right <= window.innerWidth + 1"), "the card is wider than the page"
    assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"), "horizontal page scroll"
step("phone width: the card fits", lambda: run(8932, "/tmp/wd_spc2", narrow, {"width": 390, "height": 800}))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
