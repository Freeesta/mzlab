"""Spectrum and chromatogram cards carry NO info rows between the header and the chart: no Notes, Retention times, Collision energy or Sample row
(the title has the RT, the header the sample selector, the legend the sample). The chart sits right under the header (it took the room of the rows).
The MS¹ card keeps ONE header row, the MS² card two lines (title row, then the fragments controls); an old saved spectrum note is still read without error."""
# covers: mzlab/web/spettro-righe.js, mzlab/web/app.css
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "righe_tmp"; D.mkdir(parents=True, exist_ok=True)
MS2F, FULL = D / "Stepped_t15.mzML", D / "Full_t0.mzML"
ds.ms2(MS2F, np.random.default_rng(5), 15); ds.full_scan(FULL, 0, np.random.default_rng(6))
SP2 = "E.panels.find(p => p.tab === 'ms2' && p.type === 'spec')"
CH2 = "E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom')"
SP1 = "E.panels.find(p => p.tab === 'full' && p.type === 'spec')"
CH1 = "E.panels.find(p => p.tab === 'full' && p.type === 'chrom')"
LABELS = ("Notes", "Retention times", "Collision energy", "Sample")
def no_rows(pg, card, what):
    """None of the four rows (no .srows block, no .sr / .srl element, none of the labels as a row label) and the chart starts right under the header."""
    assert pg.evaluate(f"!!{card}"), f"no {what} card"
    assert pg.evaluate(f"!{card}.el.querySelector('.srows, .sr, .srl, .sr-notes, .sr-rt, .sr-ce, .sr-smp')"), f"info rows on the {what} card"
    txt = pg.evaluate(f"{card}.el.innerText")
    for lab in LABELS: assert lab not in [l.strip() for l in txt.splitlines()], f"{what} card shows the row «{lab}»"
    assert pg.evaluate(f"!{card}.el.querySelector('input[placeholder=\"Your notes on this spectrum\"]')"), f"notes input on the {what} card"
    gap = pg.evaluate(f"{card}.cv.getBoundingClientRect().top - {card}.el.querySelector('.hd').getBoundingClientRect().bottom")
    assert gap < 16, f"{what} card: {gap:.0f}px between the header and the chart (a row is still taking the room)"
def ms2(pg):
    no_rows(pg, SP2, "MS2 spectrum"); no_rows(pg, CH2, "chromatogram")
    # the header stays two lines by design: the title row, then the fragments controls on their own line
    assert pg.evaluate(f"{SP2}.el.classList.contains('ms2c') && getComputedStyle({SP2}.el.querySelector('.hd')).flexWrap") == "wrap"
    assert pg.evaluate(f"{SP2}.el.querySelector('.ctl').getAttribute('aria-label')") == "Fragments and intensities"
    # the sample stays where it already is: the header sample selector
    assert pg.evaluate(f"!!{SP2}.el.querySelector('.hd select')"), "no sample selector in the MS2 header"
def ms1(pg):
    no_rows(pg, SP1, "MS1 spectrum"); no_rows(pg, CH1, "chromatogram")
    assert pg.evaluate(f"!{SP1}.el.classList.contains('ms2c')"), "MS1 card got the MS2 class"
    assert pg.evaluate(f"getComputedStyle({SP1}.el.querySelector('.hd')).flexWrap") == "nowrap", "MS1 header wraps onto a second line"
    hh = pg.evaluate(f"{SP1}.el.querySelector('.hd').getBoundingClientRect().height"); assert hh < 60, f"MS1 header is {hh}px tall: more than one row"
def narrow(pg):
    ms2(pg)
    assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"), "horizontal page scroll"
def old_note(pg):
    """A spectrum note saved by an older version is still in the notebook record and read back: no error, no row, the value is kept."""
    pg.evaluate(f"{SP2}.note = 'old saved note'"); saved(pg)
    pg.reload(); pg.wait_for_function("[...document.querySelectorAll('.pnl canvas')].some(c => c.checkVisibility())", timeout=120000); ready(pg)
    wait_for_js(pg, f"!!{SP2}"); ready(pg)
    assert pg.evaluate(f"{SP2}.note") == "old saved note", "the saved note was dropped"
    no_rows(pg, SP2, "restored MS2 spectrum")
def run(port, wd, f, body, size=None, tab="ms2"):
    r = Run(port=port, wd=wd)
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            if size: pg.set_viewport_size(size)
            stage(pg, [str(f)]); pg.click("text=Load data"); ready(pg)
            if tab == "ms2":
                open_ms2(pg); pg.locator('#flst [data-pg]').first.dblclick(); wait_for_js(pg, f"!!{SP2}")
            else: pg.evaluate("setTab('full')"); wait_for_js(pg, f"!!{SP1}")
            ready(pg); body(pg)
            errs = [e for e in r.errs if e[0] == "pageerror"]; assert not errs, errs
    finally: r.close()
step("MS1 card + chromatogram: no Notes / Retention times / Collision energy / Sample rows, one header row", lambda: run(8914, "/tmp/wd_row4", FULL, ms1, tab="full"))
step("MS2 card + chromatogram: no rows, two header lines, chart under the header", lambda: run(8911, "/tmp/wd_row1", MS2F, ms2))
step("phone width: no rows, no page scroll", lambda: run(8913, "/tmp/wd_row3", MS2F, narrow, {"width": 390, "height": 800}))
step("an old saved spectrum note is read back without error", lambda: run(8912, "/tmp/wd_row2", MS2F, old_note))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
