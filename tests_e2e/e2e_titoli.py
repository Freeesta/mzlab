"""Panel titles (titoli.js): the chromatogram of an MS2 experiment is a TIC and its spectrum names the precursor; MS1, MRM and PDA (single sample, overlay, one wavelength) say what they show and no chromatogram title names a file."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import numpy as np
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
D = HERE / "shots" / "titoli_tmp"; D.mkdir(parents=True, exist_ok=True)
MRMF = D / "MRM_t0.mzML"; ds.mrm(MRMF, 1.0, np.random.default_rng(4), 0)
PDA = [D / "PDA_t0.mzML", D / "PDA_t45.mzML"]
for f, t in zip(PDA, (0, 45)): ds.full_scan(f, t, np.random.default_rng(3), dad=True)
TTL = lambda q: f"({q}).el.querySelector('.ttl').textContent"
CH = "E.panels.find(q => q.type === 'chrom')"
def run(port, wd, files, body):
    r = Run(port=port, wd=wd)
    try:
        with sync_playwright() as p:
            pg = r.page(p); stage(pg, files); pg.click("text=Carica dati"); ready(pg); body(pg)
    finally: r.close()
def ms2(pg):
    open_ms2(pg); pg.locator('#flst [data-pg="229.1"]').dblclick()
    attendi(pg, "E.panels.some(p => p.tab === 'ms2' && p.type === 'spec')"); ready(pg)
    top = pg.evaluate(TTL("E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom')")); spec = pg.evaluate(TTL("E.panels.find(p => p.tab === 'ms2' && p.type === 'spec')"))
    assert top == "TIC · MS² · precursore 229.1", top        # two precursors in the files: the trace is limited to one
    assert spec == "MS² di 229.1" and top != spec, spec
    assert pg.evaluate("document.querySelector('.pnl.spec .rtl') !== null"), "retention time still shown"
    pg.evaluate("E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom').prec = null"); assert pg.evaluate("TITOLI.di(E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom'))") == "TIC · MS² (tutti i precursori)"
def ms1(pg):
    assert pg.evaluate("TITOLI.di(E.panels.find(q => q.type === 'spec')) === 'MS¹'") if pg.evaluate("E.panels.some(q => q.type === 'spec')") else True
    assert pg.evaluate("TITOLI.di(E.panels.find(q => q.type === 'chrom'))") == "TIC"
def mrm(pg):
    assert pg.evaluate("TITOLI.di(E.panels.find(q => q.type === 'mrm'))") == "MRM"
    t = pg.evaluate("E.panels.find(q => q.type === 'mrm').el.querySelector('.ttl').textContent"); assert t == "MRM", t
def pda_single(pg):
    pick_kind(pg, "pda"); ready(pg)
    assert pg.evaluate(TTL(CH)) == "PDA (tutte le λ)", pg.evaluate(TTL(CH))
    assert pg.evaluate(f"TITOLI.di(Object.assign({CH}, {{ mz0: 254, mz1: 254 }}))") == "PDA 254 nm"
def pda_overlay(pg):
    pg.evaluate("E.files.forEach(f => { f.vis = true; })"); pick_kind(pg, "pda"); ready(pg)
    t = pg.evaluate(TTL(CH)); assert t == "PDA (tutte le λ)" and "PDA_t" not in t, t
    assert pg.evaluate("![...document.querySelectorAll('.pnl.chrom .pscan')].length"), "a chromatogram header names a file"
step("MS2: TIC on top, precursor in the spectrum title", lambda: run(8901, "/tmp/wd_tit1", [str(ES / f"MS2_t{t}.mzML") for t in (15, 60)], ms2))
step("MS1: TIC / MS¹", lambda: run(8902, "/tmp/wd_tit2", [str(ES / "FullScan_t15.mzML")], ms1))
step("MRM", lambda: run(8903, "/tmp/wd_tit3", [str(MRMF)], mrm))
step("PDA single sample + one wavelength", lambda: run(8904, "/tmp/wd_tit4", [str(PDA[0])], pda_single))
step("PDA overlay: title does not name a file", lambda: run(8905, "/tmp/wd_tit5", [str(f) for f in PDA], pda_overlay))
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
sys.exit(1 if bad else 0)
