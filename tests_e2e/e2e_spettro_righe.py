"""MS² spectrum card rows (spettro-righe.js): fragments (header controls), notes, retention times, CE ± spread from the file, sample; no horizontal scroll at phone width.
An MS¹ card keeps ONE header row and none of these rows (regression of #3435, which scoped them to every spectrum card)."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import re
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "righe_tmp"; D.mkdir(parents=True, exist_ok=True)
FLAT, STEP = D / "Flat_t15.mzML", D / "Stepped_t15.mzML"
ds.ms2(FLAT, np.random.default_rng(5), 15)
ds.ms2(STEP, np.random.default_rng(5), 15)
txt, n = STEP.read_text(), [0]
def cycle(m):
    n[0] += 1; return m.group(1) + ("10.0", "25.0", "40.0")[n[0] % 3] + '"'
STEP.write_text(re.sub(r'(name="collision energy" value=")10\.0"', cycle, txt))   # stepped 10/25/40 on every precursor: 25 ± 15
SP = "E.panels.find(p => p.tab === 'ms2' && p.type === 'spec')"
ROWS = f"[...{SP}.el.querySelectorAll('.srows > .sr')].map(r => r.className.replace('sr ', '') + '=' + r.lastElementChild.textContent)"
def run(port, wd, f, body, size=None):
    r = Run(port=port, wd=wd)
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            if size: pg.set_viewport_size(size)
            stage(pg, [str(f)]); pg.click("text=Load data"); ready(pg); open_ms2(pg)
            pg.locator('#flst [data-pg]').first.dblclick()
            wait_for_js(pg, f"!!{SP}"); ready(pg); body(pg)
    finally: r.close()
def stepped(pg):
    rows = dict(x.split("=", 1) for x in pg.evaluate(ROWS))
    assert rows.get("sr-ce") == "CE 25 ± 15 eV", rows
    assert rows.get("sr-smp") == "Stepped_t15", rows
    assert rows.get("sr-rt", "").startswith("Scans "), rows
    assert "sr-notes" in rows, rows
    pg.locator(".pnl.spec .sr-notes input").fill("my note"); assert pg.evaluate(f"{SP}.note") == "my note"
    assert pg.evaluate(f"{SP}.el.querySelector('.ctl').getAttribute('aria-label')") == "Fragments and intensities"
    tips = pg.evaluate(f"[...{SP}.el.querySelectorAll('.srows [title]')].map(e => e.title.length)"); assert all(t <= 60 for t in tips), tips
def flat(pg):
    rows = dict(x.split("=", 1) for x in pg.evaluate(ROWS))
    assert rows.get("sr-ce") == "CE 10 eV" and "±" not in rows["sr-ce"], rows
def narrow(pg):
    assert pg.evaluate("document.documentElement.scrollWidth <= window.innerWidth + 1"), "horizontal page scroll"
    assert pg.evaluate(f"[...{SP}.el.querySelectorAll('.srows > .sr')].every(r => r.getBoundingClientRect().right <= {SP}.el.getBoundingClientRect().right + 1)")
MS1 = "E.panels.find(p => p.tab === 'full' && p.type === 'spec')"
def ms1_card(pg):
    """An MS¹ card on the Full-scan tab: single header row, no .srows (notes, retention times), no .ms2c class."""
    assert pg.evaluate(f"!!{MS1}"), "no MS1 spectrum card"
    assert pg.evaluate(f"!{MS1}.el.querySelector('.srows') && !{MS1}.el.classList.contains('ms2c')"), "rows on an MS1 card"
    assert pg.evaluate(f"!{MS1}.el.querySelector('.sr-notes, .sr-rt')"), "notes / retention-time row on an MS1 card"
    assert pg.evaluate(f"getComputedStyle({MS1}.el.querySelector('.hd')).flexWrap") == "nowrap", "MS1 header wraps onto a second line"
    hh = pg.evaluate(f"{MS1}.el.querySelector('.hd').getBoundingClientRect().height"); assert hh < 60, f"MS1 header is {hh}px tall: more than one row"
def run_ms1(port, wd, f):
    r = Run(port=port, wd=wd)
    try:
        with sync_playwright() as p:
            pg = r.page(p)
            stage(pg, [str(f)]); pg.click("text=Load data"); ready(pg); pg.evaluate("setTab('full')"); ready(pg)
            wait_for_js(pg, f"!!{MS1}"); ready(pg); ms1_card(pg)
    finally: r.close()
FULL = D / "Full_t0.mzML"; ds.full_scan(FULL, 0, np.random.default_rng(6))
step("MS1 card: one header row, no notes / retention-time rows", lambda: run_ms1(8914, "/tmp/wd_row4", FULL))
step("stepped CE: rows, CE 25 ± 15, sample, notes, aria, tooltips", lambda: run(8911, "/tmp/wd_row1", STEP, stepped))
step("single CE: CE only, no spread", lambda: run(8912, "/tmp/wd_row2", FLAT, flat))
step("phone width: rows wrap, no page scroll", lambda: run(8913, "/tmp/wd_row3", STEP, narrow, {"width": 390, "height": 800}))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
