"""MS2 sidebar: the precursor rows are per (file, precursor); ticking the m/z of one file never ticks the same m/z under another file (two MS2 files hold the same precursors)"""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import shutil, tempfile
from pathlib import Path
ES = ROOT / "mzlab" / "web" / "esempi"
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL: " + str(e).split("\n")[0][:300]))
r = Run(port=8901, wd="/tmp/wd_ms2rigafile")
try:
    with sync_playwright() as p:
        pg = r.page(p)
        # two files that really hold the same precursors: one example file under two names
        twin = Path(tempfile.mkdtemp(prefix="ms2twin_"))
        for n in ("a", "b"): shutil.copy(ES / "MS2_t15.mzML", twin / f"MS2_{n}.mzML")
        stage(pg, [str(twin / f"MS2_{n}.mzML") for n in ("a", "b")])
        pg.click("text=Carica dati"); ready(pg)
        def rows():
            pg.evaluate("setTab('ms2')"); ready(pg)
            pg.evaluate("E.files.forEach(f => { f.vis = true; }); renderFileList()")
            ids = pg.evaluate("[...document.querySelectorAll('#flst input[data-pr]')].map(x => x.id)")
            assert len(ids) == len(set(ids)) and len(ids) >= 2, f"every precursor row has its own id: {ids}"
            two = pg.evaluate("(() => { const m = {}; document.querySelectorAll('#flst input[data-pr]').forEach(x => (m[x.dataset.pr] = m[x.dataset.pr] || []).push(x.dataset.pk)); return Object.entries(m).find(([, v]) => v.length > 1); })()")
            assert two, "a precursor listed under two files"
            pr, ks = two
            sel = lambda k: f"#flst input[data-pr='{pr}'][data-pk='{k}']"
            pg.check(sel(ks[0])); ready(pg)
            assert pg.is_checked(sel(ks[0])), "the row ticked stays ticked"
            assert not pg.is_checked(sel(ks[1])), "the same m/z under the other file is not ticked"
            assert pg.evaluate(f"E.panels.find(p => p.tab === 'ms2' && p.type === 'chrom' && String(p.prec) === '{pr}').ks.length") == 1, "the chromatogram draws that file only"
            pg.check(sel(ks[1])); ready(pg)
            assert pg.is_checked(sel(ks[0])) and pg.is_checked(sel(ks[1])), "both ticked"
            pg.uncheck(sel(ks[0])); ready(pg)
            assert not pg.is_checked(sel(ks[0])) and pg.is_checked(sel(ks[1])), "unticking one leaves the other"
            pg.uncheck(sel(ks[1])); ready(pg)
            assert pg.evaluate(f"!E.panels.some(p => p.tab === 'ms2' && p.type === 'chrom' && String(p.prec) === '{pr}')"), "no row ticked: the chromatogram is closed"
        step("MS2 precursor rows are per file", rows)
finally:
    r.close()
bad = [s for s in steps if s[1] != "ok"]
for s in steps: print(s)
r.report() if bad else None
sys.exit(1 if bad else 0)
