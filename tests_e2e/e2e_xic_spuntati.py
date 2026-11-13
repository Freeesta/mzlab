"""«+ XIC» / «+ chromatogram» opened while the active experiment tab shows no file (the lone MS2 tab of a DDA file) land on the tab of the first ticked file
and draw one trace per ticked file, in the colours of the file list; with no file ticked of the panel's type the empty hint says so, and the «n/N» counter
says what it counts. Synthetic low-resolution files only."""
import sys, os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lib import *
import pathlib
steps = []
def step(name, fn):
    try: fn(); steps.append((name, "ok"))
    except Exception as e: steps.append((name, "FAIL " + str(e).split("\n")[0][:240]))
sys.path.insert(0, str(ROOT / "tools")); import dati_sintetici as ds, numpy as np
D = pathlib.Path(__file__).resolve().parent / "shots" / "xic_spuntati_tmp"; D.mkdir(parents=True, exist_ok=True)
FILES = [D / f"FS_{c}-t{t}.mzML" for c, t in (("A", 0), ("B", 10), ("C", 20))]
for i, f in enumerate(FILES):
    if not f.exists(): ds.full_scan(f, 5.0 * i, np.random.default_rng(11 + i))
IDA = D / "IDA_X-t0.mzML"
if not IDA.exists(): ds.mixed_ida(IDA, np.random.default_rng(99))
NEW = "ids => E.panels.filter(q => !ids.includes(q.id))"
r = Run(port=8871, wd="/tmp/wd_xicspunt")
try:
    with sync_playwright() as p:
        pg = r.page(p); pg.set_viewport_size({"width": 1500, "height": 1200})
        stage(pg, FILES + [IDA]); open_first(pg); ready(pg)
        st = {}
        def setup():
            # two plain full-scan files ticked (a TIC overlay), the MS2 tab of the DDA file active with its one file unticked
            ks = pg.evaluate("E.files.filter(f => f.kind === 'full' && !f.label.startsWith('IDA')).map(f => f.k)")
            for k in ks[:2]: pg.locator(f"#flst input[data-k='{k}']").check(); ready(pg)
            for k in pg.evaluate("E.files.filter(f => f.vis).map(f => f.k)"):
                if k not in ks[:2]: pg.evaluate("k => { E.files[k].vis = false; renderFileList(); renderNav(); redrawAll(); }", k)
            pg.evaluate("setTab('ms2', false, 'test')"); ready(pg)
            assert pg.evaluate("E.tab") == "ms2" and pg.evaluate("tabFiles().every(f => !f.vis)")
            st["want"] = sorted(pg.evaluate("E.files.filter(f => f.vis).map(f => [f.k, f.color])"))
            assert [k for k, _ in st["want"]] == sorted(ks[:2]), st["want"]
        step("two full-scan files ticked, the MS2 tab of the DDA file active and empty", setup)
        def add_xic():
            ids = pg.evaluate("E.panels.map(q => q.id)")
            pg.click("#np-xic"); wait_for_js(pg, "document.querySelector('#xicdlg').open")
            pg.fill("#xic-mz", "229.1"); pg.click("#xic-go")
            wait_for_js(pg, f"(({NEW})({ids})).some(q => q.type === 'xic' && q._a)"); ready(pg)
            x = pg.evaluate(f"ids => {{ const q = ({NEW})(ids).find(q => q.type === 'xic'); return {{ tab: q.tab, now: E.tab, sr: q._a.sr.map(s => [s.k, s.color, s.x.length]) }}; }}", ids)
            assert x["tab"] == "full" and x["now"] == "full", x
            assert sorted([k, c] for k, c, _ in x["sr"]) == st["want"] and all(n > 0 for *_, n in x["sr"]), (x, st["want"])
        step("«+ XIC» lands on the tab of the ticked files: one trace per ticked file, list colours", add_xic)
        def add_chrom():
            pg.evaluate("setTab('ms2', false, 'test')"); ready(pg)
            ids = pg.evaluate("E.panels.map(q => q.id)")
            pg.click("#np-chrom"); wait_for_js(pg, f"(({NEW})({ids})).some(q => q.type === 'chrom' && q._a)"); ready(pg)
            c = pg.evaluate(f"ids => {{ const q = ({NEW})(ids).find(q => q.type === 'chrom'); return {{ tab: q.tab, ks: q._a.sr.map(s => s.k).sort() }}; }}", ids)
            assert c["tab"] == "full" and pg.evaluate("E.tab") == "full" and c["ks"] == [k for k, _ in st["want"]], c
        step("«+ chromatogram» does the same", add_chrom)
        def counter():
            t = pg.evaluate("[...document.querySelectorAll('.pnl.xic .fcount')].map(b => [b.textContent.trim(), b.title])")
            want = pg.evaluate("(() => { const t = tabFiles('full'); return t.filter(f => f.vis).length + '/' + t.length; })()")
            assert t and t[0][0].startswith(want), (t, want)
            assert t[0][1] == pg.evaluate("I18N.t('ctl.files.title')") and len(t[0][1]) <= 90, t
        step("the «n/N» counter: a tooltip that says what it counts", counter)
        def hint():
            # nothing of the panel's type ticked: the hint says so (not «type an m/z»)
            q = pg.evaluate("(() => { const q = E.panels.find(q => q.type === 'xic'); tabFiles('full').forEach(f => { f.vis = false; }); draw(q); return q.id; })()")
            wait_for_js(pg, f"E.panels.find(q => q.id === {q}) && !E.panels.find(q => q.id === {q})._a")
            assert pg.evaluate(f"msgOf(E.panels.find(q => q.id === {q}))") == pg.evaluate("I18N.t('msg.noFiles')")
        step("no file of the type ticked: the empty hint says no file of this experiment type is shown", hint)
    r.close()
except Exception as e:
    steps.append(("run", "FAIL " + str(e)[:300])); r.close()
for s in steps: print(" ", s)
sys.exit(1 if [s for s in steps if s[1] != "ok"] else 0)
